#![no_std]

extern crate alloc;
use no_std_net::{ Ipv4Addr, Ipv6Addr };
use bytes::{ Bytes, BytesMut, Buf };
use percent_encoding::{ CONTROLS, AsciiSet, utf8_percent_encode };

const FRAGMENT_ENCODE: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
const QUERY_ENCODE: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'#');
const SPECIAL_QUERY_ENCODE: &AsciiSet = &QUERY_ENCODE.add(b'\'');
const PATH_ENCODE: &AsciiSet = &QUERY_ENCODE.add(b'?').add(b'`').add(b'{').add(b'}');
const USERINFO_ENCODE: &AsciiSet = &PATH_ENCODE
    .add(b'/').add(b':').add(b';').add(b'=').add(b'@').add(b'[').add(b'^').add(b'|');

pub struct Url {
    // percent_encoded: Bytes,
    serialized: Bytes,
    scheme_end: usize,
    authority_end: usize,
    host_end: usize,
    path_end: usize,
    query_end: usize,
}

impl From<&'static str> for Url {
    fn from(value: &'static str) -> Self {
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
pub enum ParseError {}

fn url_percent_encode(input: & str, encoding: &'static AsciiSet) -> Bytes {
    let mut buf = BytesMut::with_capacity(input.len());
    buf.extend(utf8_percent_encode(input, encoding).flat_map(|s| s.as_bytes()));
    buf.freeze()
}

// TODO: manage static lifetime here
pub fn parse_url(input: &'static str) -> Result<Url, ParseError> {
    let scheme_end = input.find(':').unwrap_or(0);
    let authority_end = input[scheme_end..].find('@').unwrap_or(0);
    let host_end = input[authority_end..].find([':', '/']).unwrap_or(scheme_end);
    let path_end = input[host_end..].find('?').unwrap_or(host_end);
    let query_end = input[path_end..].find('#').unwrap_or(path_end);

    Ok(Url {
        serialized: Bytes::from(input),
        scheme_end,
        authority_end,
        host_end,
        path_end,
        query_end,
    })
}

impl Url {
    pub fn scheme(&self) -> &str {
        core::str::from_utf8(&self.serialized[..self.scheme_end]).unwrap()
    }
    pub fn host_serialized(&self) -> &str {
        core::str::from_utf8(&self.serialized[self.authority_end+1..self.host_end]).unwrap()
    }
    // pub fn serialize(&self) -> &'_ str {
    //     let mut percent_encoded = BytesMut::with_capacity(input.len());
    //     percent_encoded.extend(url_percent_encode(&input[..scheme_end], CONTROLS)
    //         .chain(url_percent_encode(&input[scheme_end..authority_end], USERINFO_ENCODE))
    //         .chain(url_percent_encode(&input[authority_end..host_end], CONTROLS))
    //         .chain(
    //             input[host_end..path_end].split('/')
    //                 .flat_map(|s| url_percent_encode(s, PATH_ENCODE).slice(..)).collect::<Bytes>()
    //         ));
    //     let percent_encoded = percent_encoded.freeze();
    // }
}
