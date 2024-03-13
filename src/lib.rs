#![cfg_attr(not(test), no_std)]

extern crate alloc;
// use no_std_net::{ Ipv4Addr, Ipv6Addr };
use bytes::{ Bytes, BytesMut, BufMut };
use percent_encoding::{ CONTROLS, AsciiSet, utf8_percent_encode, percent_decode };

const FRAGMENT_ENCODE: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
const QUERY_ENCODE: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'#');
const SPECIAL_QUERY_ENCODE: &AsciiSet = &QUERY_ENCODE.add(b'\'');
const PATH_ENCODE: &AsciiSet = &QUERY_ENCODE.add(b'?').add(b'`').add(b'{').add(b'}');
const USERINFO_ENCODE: &AsciiSet = &PATH_ENCODE
    .add(b'/').add(b':').add(b';').add(b'=').add(b'@').add(b'[').add(b'^').add(b'|');

pub struct Url {
    serialized: Bytes,
    scheme_end: usize,
    userinfo_end: usize,
    host_end: usize,
    path_end: usize,
    query_end: usize,
}

impl From<&str> for Url {
    fn from(value: &str) -> Self {
        parse_url(value).unwrap()
    }
}

impl core::str::FromStr for Url {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_url(s)
    }
}

#[derive(Debug)]
pub enum ParseError {
    SpecialSchemeMissingFollowingSolidus,
}

// TODO: manage static lifetime here
pub fn parse_url(input: &str) -> Result<Url, ParseError> {
    assert!(!input.is_empty());

    let input = input.trim();
    let mut buf = BytesMut::with_capacity(input.len());

    match input.get(0..1).unwrap() {
        c if c.is_ascii() => {
            let scheme_end = parse_scheme(input, &mut buf).unwrap();
            match &input[..scheme_end] {
                // "file" => {},// TODO: implement file host parsing
                "http" | "https" | "ws" | "wss" | "ftp" => {
                    if !&input[scheme_end+1..].starts_with("//") {
                        return Err(ParseError::SpecialSchemeMissingFollowingSolidus);
                    }
                    buf.put_slice(b"://");
                    let userinfo_end = parse_userinfo(input, &mut buf, scheme_end+3).unwrap();
                    let buf_userinfo_end = buf.len();
                    let host_end = parse_host(input, &mut buf, userinfo_end).unwrap();
                    let buf_host_end = buf.len();

                    if host_end >= input.len() {
                        // TODO: adjust returned values for the percent encoded buffer (may add return args with adjusted buffer lengths)
                        return Ok(Url {
                            serialized: buf.freeze(), scheme_end,
                            userinfo_end: buf_userinfo_end,
                            host_end: buf_host_end,
                            path_end: buf_host_end, query_end: buf_host_end
                        });
                    }
                    // TODO: parse ports
                    let path_end = parse_path(input, &mut buf, host_end).unwrap();
                    let buf_path_end = buf.len();

                    if path_end >= input.len() {
                        // TODO: adjust returned values for the percent encoded buffer (may add return args with adjusted buffer lengths)
                        return Ok(Url {
                            serialized: buf.freeze(), scheme_end,
                            userinfo_end: buf_userinfo_end,
                            host_end: buf_host_end,
                            path_end: buf_path_end,
                            query_end: buf_path_end
                        });
                    }

                    let query_end = parse_query(input, &mut buf, path_end).unwrap();
                    let buf_query_end = buf.len();

                    if buf_query_end < input.len() {
                        buf.extend(
                        utf8_percent_encode(&input[query_end..], FRAGMENT_ENCODE)
                            .flat_map(|s| s.as_bytes()
                            )
                        );
                    }

                    Ok(Url {
                        serialized: buf.freeze(),
                        scheme_end,
                        userinfo_end: buf_userinfo_end,
                        host_end: buf_host_end,
                        path_end: buf_path_end,
                        query_end: buf_query_end,
                    })
                },
                _ => todo!()
            }
        },
        _ => todo!()
    }
}

fn parse_scheme(input: &str, buf: &mut BytesMut) -> Result<usize, ()> {
    let end = input.find(':').unwrap_or(0);

    for c in input[..end].chars() {
        if c.is_alphabetic() || c=='+' || c=='-' || c=='.' { buf.put_u8((c.to_ascii_lowercase()) as u8)}
        else { buf.clear(); return Err(()); }
    }

    Ok(end)
}

fn parse_userinfo(input: &str, buf: &mut BytesMut, scheme_end: usize) -> Result<usize, ()> {
    let userinfo_end = input[scheme_end..].find('@').unwrap_or(scheme_end);
    let (username, password) = input[scheme_end..userinfo_end].split_once(':').unwrap_or(("", ""));

    if !username.is_empty() {
        buf.extend(utf8_percent_encode(username, USERINFO_ENCODE).flat_map(|s| s.as_bytes()));
        if !password.is_empty() {
            buf.put_u8(b':');
            buf.extend(utf8_percent_encode(password, USERINFO_ENCODE).flat_map(|s| s.as_bytes()));
        }
        buf.put_u8(b'@');
    }

    Ok(userinfo_end)
}

fn parse_host(input: &str, buf: &mut BytesMut, userinfo_end: usize) -> Result<usize, ()> {
    // TODO: parse ipv6 + ipv4 addresses
    if input[userinfo_end..].starts_with(['/', '?', '#']) { return Err(()); } // return host-missing parse error here

    let host_end = match input[userinfo_end..].find(['/', '?', '#']) {
        Some(b) => userinfo_end + b, None => input.len()
    };

    buf.put_slice(input[userinfo_end..host_end].as_bytes());
    Ok(host_end)
}

fn parse_path(input: &str, buf: &mut BytesMut, host_end: usize) -> Result<usize, ()> {
    if input[host_end..].starts_with(['?', '#']) { return Ok(host_end); }

    let path_end = match input[host_end..].find(['?', '#']) {
        Some(b) => host_end + b, None => input.len()
    };
    // adding one to host end here jumps over the initial slash, which removes the first match (empty segment)
    let path_segments = input[host_end..path_end].split('/');

    for segment in path_segments {
        if !segment.is_empty() {
            if !buf.ends_with(b"/") { buf.put_u8(b'/'); }
            buf.extend(utf8_percent_encode(segment, PATH_ENCODE).flat_map(|s| s.as_bytes()));
        } else if !buf.ends_with(b"/") {
            buf.put_u8(b'/');
        } else {
            continue;
        }
    }
    Ok(path_end)
}

fn parse_query(input: &str, buf: &mut BytesMut, path_end: usize) -> Result<usize, ()> {
    if input[path_end..].starts_with('#') { return Ok(path_end); }

    let query_end = match input[path_end..].find('#') {
        Some(b) => path_end + b, None => input.len()
    };
    buf.extend(utf8_percent_encode(&input[path_end..query_end], SPECIAL_QUERY_ENCODE).flat_map(|s| s.as_bytes()));
    Ok(query_end)
}

fn bytes_to_str(bytes: &[u8]) -> &str {
    core::str::from_utf8(bytes).unwrap()
}

impl Url {
    pub fn scheme(&self) -> &str {
        bytes_to_str(&self.serialized[..self.scheme_end])
    }
    pub fn authority(&self) -> &str {
        if self.serialized[self.scheme_end..].starts_with(b"://") {
            bytes_to_str(&self.serialized[self.scheme_end+3..self.host_end])
        } else {
            bytes_to_str(&self.serialized[self.scheme_end+1..self.host_end])
        }
    }
    pub fn host(&self) -> &str {
        bytes_to_str(&self.serialized[self.userinfo_end..self.host_end])
    }
    pub fn path_and_query(&self) -> &str {
        bytes_to_str(&self.serialized[self.host_end..self.query_end])
    }
    pub fn path(&self) -> &str {
        bytes_to_str(&self.serialized[self.host_end..self.path_end])
    }
    pub fn query(&self) -> &str {
        bytes_to_str(&self.serialized[self.path_end..self.query_end])
    }
    pub fn fragment(&self) -> &str {
        bytes_to_str(&self.serialized[self.query_end..])
    }
    pub fn as_str(&self) -> alloc::borrow::Cow<'_, str> {
        let percent_decoded = percent_decode(&self.serialized).decode_utf8().unwrap();
        percent_decoded
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn basic_parse_url() {
        let url = "http://www.example.com";
        assert_eq!(url, parse_url(url).unwrap().as_str());
    }
    #[test]
    fn path_parse_url() {
        let url = "http://www.example.com/doc/glossary";
        assert_eq!(url, parse_url(url).unwrap().as_str());
    }
    #[test]
    fn path_and_query_parse_url() {
        let url = "http://www.example.com/scripts/?job=111&task=1";
        assert_eq!(url, parse_url(url).unwrap().as_str());
    }
    #[test]
    fn single_slash_path_parse_url() {
        let url = "http://www.example.com/";
        assert_eq!(url, parse_url(url).unwrap().as_str());
    }
    #[test]
    fn fragment_parse_url() {
        let url = "http://www.example.com#introduction";
        assert_eq!(url, parse_url(url).unwrap().as_str());
    }
}
