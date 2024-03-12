#![no_std]

extern crate alloc;
use core::ops::Range;
use no_std_net::{ Ipv4Addr, Ipv6Addr };
use bytes::{ Bytes, BytesMut, Buf };
use percent_encoding::{ CONTROLS, AsciiSet, percent_encode };

const FRAGMENT_ENCODE: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
const QUERY_ENCODE: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'#');
const SPECIAL_QUERY_ENCODE: &AsciiSet = &QUERY_ENCODE.add(b'\'');
const PATH_ENCODE: &AsciiSet = &QUERY_ENCODE.add(b'?').add(b'`').add(b'{').add(b'}');
const USERINFO_ENCODE: &AsciiSet = &PATH_ENCODE
    .add(b'/').add(b':').add(b';').add(b'=').add(b'@').add(b'[').add(b'^').add(b'|');

pub struct Url {
    percent_encoded: Bytes,
    scheme: Range<usize>,
    authority: Range<usize>,
    host: Range<usize>,
    // port: Range<usize>,
    path: Range<usize>,
    query: Range<usize>,
    fragment: Range<usize>,
}

impl core::str::FromStr for Url {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_url(s)
    }
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
pub enum ParseError {}

// TODO: make this return a &str
fn url_percent_encode(input: &'_ str, encoding: &'static AsciiSet) -> Bytes {
    let mut buf = BytesMut::with_capacity(input.len());
    buf.extend(percent_encode(input.as_bytes(), encoding).flat_map(|s| s.as_bytes()));
    buf.freeze()
}

pub fn parse_url(input: &'_ str) -> Result<Url, ParseError> {
    let scheme_end = input.find(':').unwrap_or(0);
    let authority_end = input[scheme_end..].find('@').unwrap_or(0);
    let host_end = input[authority_end..].find([':', '/']).unwrap_or(scheme_end);
    // let port_end = input[host_end..host_end+5].find('/').unwrap_or(host_end);
    // let path_end = input[host_end..].find('?').unwrap_or(port_end);
    let path_end = input[host_end..].find('?').unwrap_or(host_end);
    let query_end = input[path_end..].find('#').unwrap_or(path_end);

    let mut percent_encoded = BytesMut::with_capacity(input.len());
    percent_encoded.extend(url_percent_encode(&input[..scheme_end], CONTROLS)
        .chain(url_percent_encode(&input[scheme_end..authority_end], USERINFO_ENCODE))
        .chain(url_percent_encode(&input[authority_end..host_end], CONTROLS))
        .chain(
            input[host_end..path_end].split('/')
                .flat_map(|s| url_percent_encode(s, PATH_ENCODE).slice(..)).collect::<Bytes>()
        ));
    let percent_encoded = percent_encoded.freeze();
    let percent_encoded_len = percent_encoded.len();

    Ok(Url {
        percent_encoded,
        scheme: 0..scheme_end,
        authority: scheme_end..authority_end,
        host: authority_end..host_end,
        // port: host_end..port_end,
        // path: port_end..path_end,
        path: host_end..path_end,
        query: path_end..query_end,
        fragment: query_end..percent_encoded_len
    })
}
