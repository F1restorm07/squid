#![no_std]

extern crate alloc;
use no_std_net::{ Ipv4Addr, Ipv6Addr };
use bytes::{ Bytes, BytesMut, Buf, BufMut };
use percent_encoding::{ CONTROLS, AsciiSet, utf8_percent_encode };

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

pub enum Scheme<'s> {
    Http,
    Https,
    File,
    Ftp,
    Ws,
    Wss,
    Other(&'s str)
}

pub struct Authority<'a> {
    user: &'a str,
    password: &'a str,
}

pub struct Host<'h> {
    name: Hostname<'h>,
    domain: Option<&'h str>,
}

pub enum Hostname<'n> {
    DnsDomain(&'n str),
    IPv4(Ipv4Addr),
    IPv6(Ipv6Addr),
}

pub struct Query<'q>(&'q str, &'q str);

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
                    let userinfo_end = parse_userinfo(&input[userinfo_start..], &mut buf).unwrap();
                    let host_end = parse_host(&input[userinfo_end+1..], &mut buf).unwrap();
                    // TODO: parse ports
                    let path_end = parse_path(&input[host_end..], &mut buf).unwrap();
                    let query_end = parse_query(&input[path_end..], &mut buf).unwrap();
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
        _ => {todo!()} // TODO: fill this out
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

fn parse_userinfo(input: &str, buf: &mut BytesMut) -> Result<usize, ()> {
    let authority_end = input.find('@').unwrap_or(buf.len());

    let (username, password) = input[..authority_end].split_once(':').unwrap_or((&input[..authority_end], ""));
    buf.extend(utf8_percent_encode(username, USERINFO_ENCODE).flat_map(|s| s.as_bytes()));
    if !password.is_empty() {
        buf.put_u8(b':');
        buf.extend(utf8_percent_encode(password, USERINFO_ENCODE).flat_map(|s| s.as_bytes()));
    }
    buf.put_u8(b'@');
    Ok(authority_end) // ??: should i return the input's end or the buffer length (potentially much different)
}

fn parse_host(input: &str, buf: &mut BytesMut) -> Result<usize, ()> {
    // TODO: parse ipv6 + ipv4 addresses
    if input.starts_with(['/', '?', '#']) { return Err(()); } // return host-missing parse error here
    let host_end = input.find(['/', '?', '#']).unwrap_or(buf.len());
    buf.put_slice(input[..host_end].as_bytes());
    Ok(host_end)
}

fn parse_path(input: &str, buf: &mut BytesMut) -> Result<usize, ()> {
    if input.starts_with(['?', '#']) { return Ok(buf.len()); }
    let path_end = input.find(['?', '#']).unwrap_or(usize::MAX);
    // ??: ^^ is there a better way to denote that the path is the last url component
    let path_segments = input[1..path_end].split('/');

    for segment in path_segments {
        buf.put_u8(b'/');
        buf.extend(utf8_percent_encode(segment, PATH_ENCODE).flat_map(|s| s.as_bytes()));
    }

    Ok(path_end)
}

fn parse_query(input: &str, buf: &mut BytesMut) -> Result<usize, ()> {
    let query_end = input.find('#').unwrap_or(usize::MAX);
    buf.extend(utf8_percent_encode(&input[..query_end], SPECIAL_QUERY_ENCODE).flat_map(|s| s.as_bytes()));
    Ok(query_end) // ??: may return Option<usize> to accomodate if find() returns None
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
}
