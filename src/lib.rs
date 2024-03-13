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
                    let userinfo_start = buf.len();
                    let userinfo_end = parse_userinfo(input, &mut buf, scheme_end+3).unwrap();
                    let host_end = parse_host(input, &mut buf, userinfo_end).unwrap();

                    #[cfg(test)]
                    println!("host_end: {host_end}, input_len: {}", input.len());

                    if host_end >= input.len() {
                        // TODO: adjust returned values for the percent encoded buffer (may add return args with adjusted buffer lengths)
                        return Ok(Url {
                            serialized: buf.freeze(), scheme_end, userinfo_end, host_end,
                            path_end: host_end, query_end: host_end
                        });
                    }
                    // TODO: parse ports
                    let path_end = parse_path(input, &mut buf, host_end).unwrap();

                    if path_end >= input.len() {
                        // TODO: adjust returned values for the percent encoded buffer (may add return args with adjusted buffer lengths)
                        return Ok(Url {
                            serialized: buf.freeze(), scheme_end, userinfo_end, host_end, path_end,
                            query_end: path_end
                        });
                    }

                    let query_end = parse_query(&input[path_end..], &mut buf, path_end).unwrap();
                    buf.extend(utf8_percent_encode(&input[query_end..], FRAGMENT_ENCODE).flat_map(|s| s.as_bytes()));

                    Ok(Url {
                        serialized: buf.freeze(),
                        scheme_end,
                        userinfo_end,
                        host_end,
                        path_end,
                        query_end,
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
    if input.starts_with(['/', '?', '#']) { return Err(()); } // return host-missing parse error here
    let host_end = input[userinfo_end..].find(['/', '?', '#']).unwrap_or(input.len());

    #[cfg(test)]
    println!("idx: {host_end}, full: {input}");

    buf.put_slice(input[userinfo_end..host_end].as_bytes());
    Ok(host_end)
}

fn parse_path(input: &str, buf: &mut BytesMut, host_end: usize) -> Result<usize, ()> {
    if input.starts_with(['?', '#']) { return Ok(host_end); }
    let path_end = input[host_end..].find(['?', '#']).unwrap_or(host_end);
    let path_segments = input[host_end..path_end].split('/');

    for segment in path_segments {
        if !segment.is_empty() {
            buf.put_u8(b'/');
            buf.extend(utf8_percent_encode(segment, PATH_ENCODE).flat_map(|s| s.as_bytes()));
        }
    }
    Ok(path_end)
}

fn parse_query(input: &str, buf: &mut BytesMut, path_end: usize) -> Result<usize, ()> {
    let query_end = input[path_end..].find('#').unwrap_or(path_end);
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
    pub fn as_str(&self) -> alloc::borrow::Cow<'_, str> {
        let percent_decoded = percent_decode(&self.serialized).decode_utf8().unwrap();
        percent_decoded
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // #[test]
    // fn basic_parse_host() {
    //     let host = "www.example.com";
    //     let mut buf = BytesMut::with_capacity(host.len());
    //     let _ = parse_host(host, &mut buf).unwrap();
    //     assert_eq!(
    //         host,
    //         bytes_to_str(&buf[..])
    //     )
    // }
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
}
