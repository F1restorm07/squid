#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use no_std_net::{ Ipv4Addr, Ipv6Addr };

pub struct Url<'u> {
    scheme: Scheme<'u>,
    authority: Option<Authority<'u>>,
    host: Option<Host<'u>>,
    port: Option<u16>,
    // TODO: change to a static array or slice
    path: Option<Vec<&'u str>>,
    query: Option<Vec<Query<'u>>>,
    fragment: Option<&'u str>,
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

pub fn parse_url(input: &'_ str) -> Result<Url<'_>, ParseError> {
    let (scheme, rest) = parse_scheme(input).unwrap();
    let (authority, rest) = parse_authority(rest).unwrap();
    let (host, rest) = parse_host(rest).unwrap();
    let (port, rest) = parse_port(rest).unwrap();
    let port = if port.is_none() {
      match scheme {
        Scheme::Http | Scheme::Ws => Some(80),
        Scheme::Https | Scheme::Wss => Some(443),
        Scheme::Ftp => Some(21),
        Scheme::File | Scheme::Other(_) => None
        }
    } else { port };
    let (path, rest) = parse_path(rest).unwrap();
    let (query, fragment) = parse_query(rest).unwrap();

    Ok(Url { scheme, authority, host, port, path, query, fragment })
}

fn parse_scheme(input: &'_ str) -> Result<(Scheme<'_>, &'_ str), ParseError> {
    let input = input.split_once(':').unwrap();
    let scheme = match input.0 {
        "http" => Scheme::Http,
        "https" => Scheme::Https,
        "file" => Scheme::File,
        "ftp" => Scheme::Ftp,
        "ws" => Scheme::Ws,
        "wss" => Scheme::Wss,
        s => Scheme::Other(s)
    };
    Ok((scheme, input.1.strip_prefix("//").unwrap_or("")))
}

fn parse_authority(input: &'_ str) -> Result<(Option<Authority>, &'_ str), ParseError> {
    let input = input.split_once('@').unwrap_or(("", input));
    if input.0.is_empty() { return Ok((None, input.1)) }

    let (user, password) = input.0.split_once(':').unwrap();
    Ok((Some(Authority { user, password }), input.1))
}

fn parse_host(input: &'_ str) -> Result<(Option<Host<'_>>, &'_ str), ParseError> {
    let input = input.split_once('/').unwrap_or(("", input));
    if input.0.is_empty() { return Ok((None, input.1)) }

    let (hostname, domain) = input.0.split_once('.').unwrap();
    Ok((Some(Host { name: Hostname::DnsDomain(hostname), domain: Some(domain) }), input.1))
}

fn parse_port(input: &'_ str) -> Result<(Option<u16>, &'_ str), ParseError> {
    let input = input.split_once('/').unwrap_or(("", input));
    if input.0.is_empty() { return Ok((None, input.1)) }

    Ok((Some(input.0.parse().unwrap()), input.1))
}

fn parse_path(input: &'_ str) -> Result<(Option<Vec<&'_ str>>, &'_ str), ParseError> {
    let input = input.split_once('?').unwrap_or(("", input));
    if input.0.is_empty() { return Ok((None, input.1)) }

    // ??: how do i collect directly into a slice
    let path = input.0.split("/").collect::<Vec<_>>();
    Ok((Some(path), input.1))
}

fn parse_query(input: &'_ str) -> Result<(Option<Vec<Query<'_>>>, Option<&'_ str>), ParseError> {
    let input = input.split_once('#').unwrap_or((input, ""));
    let query = input.0.split(",").map(|q| {
        let q = q.split_once('=').unwrap();
        Query(q.0, q.1)
    }).collect::<Vec<_>>();
    Ok((Some(query), if input.1.len() > 1 { Some(input.1) } else { None }))
}
