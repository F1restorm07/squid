#![no_std]

// TODO: add all url validation errors and parse errors

pub struct Url<'u> {
    url: &'u str,
    scheme_end: usize,
    userinfo_end: usize,
    host_end: usize,
    authority_end: usize,
    port: u16,
    path_end: usize,
    query_end: usize,
}

impl<'u> From<&'u str> for Url<'u> {
    fn from(value: &'u str) -> Self {
        parse_url(value).unwrap()
    }
}

impl core::fmt::Display for Url<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.url)
    }
}

impl<'u> From<Url<'u>> for &'u str {
    fn from(value: Url<'u>) -> Self {
        value.url
    }
}

#[derive(Debug)]
pub enum ParseError {
    SpecialSchemeMissingFollowingSolidus,
}

// TODO: support http origin form (path and query only)
pub fn parse_url(input: &str) -> Result<Url, ParseError> {
    assert!(!input.is_empty());

    let input = input.trim();
    let mut offset;

    // TODO: parse origin form here
    match input.get(0..1).unwrap() {
        // absolute form?
        c if c.is_ascii() => {
            let scheme = parse_scheme(input).unwrap();
            match scheme {
                "http" | "https" | "ws" | "wss" | "ftp" => {
                    if !&input[scheme.len()+1..].starts_with("//") {
                        return Err(ParseError::SpecialSchemeMissingFollowingSolidus);
                    }
                    offset = scheme.len()+3;

                    let userinfo_end = parse_userinfo(input, offset).unwrap();
                    offset = userinfo_end;

                    let (host_end, authority_end, port) = parse_host(input, offset).unwrap();
                    offset = authority_end;

                    let path_end = parse_path(input, offset).unwrap();
                    offset = path_end;

                    let query_end = parse_query(input, offset).unwrap();

                    // TODO: add default ports here
                    Ok(Url {
                        url: input, scheme_end: scheme.len(), userinfo_end, host_end, authority_end,
                        port, path_end, query_end,
                    })
                },
                "file" => todo!(),
                _ => todo!()
            }
        },
        // asterisk origin form
        "*" => Ok(Url {
            url: "*", scheme_end: 0, userinfo_end: 0, host_end: 0, authority_end: 0,
            port: 80, path_end: 0, query_end: 0
        }),
        // origin form
        "/" => todo!(),
        _ => todo!()
    }
}

fn parse_scheme(input: &str) -> Result<&str, ()> {
    let end = input.find(':').unwrap_or(0);

    Ok(&input[..end])
}

fn parse_userinfo(input: &str, offset: usize) -> Result<usize, ()> {
    let userinfo_end = input[offset..].find('@').unwrap_or(offset);
    // let (username, password) = input[offset..userinfo_end].split_once(':').unwrap_or(("", ""));
    // TODO: check if i should skip over the password or not
    Ok(userinfo_end)
}

fn parse_host(input: &str, offset: usize) -> Result<(usize, usize, u16), ()> {
    // TODO: parse ipv6 + ipv4 addresses
    if input[offset..].starts_with(['?', '#']) { return Err(()); }
    let host_end = match input[offset..].find(['/', '?', '#']) {
        Some(b) => offset + b, None => input.len()
    };
    let (host, port) = input[offset..host_end].split_once(':').unwrap_or(("", ""));
    let port_num: u16 = port.parse().unwrap_or(0);

    Ok((offset + host.len(), host_end, port_num))
}

fn parse_path(input: &str, offset: usize) -> Result<usize, ()> {
    if offset >= input.len() { return Ok(offset); }

    let path_end = match input[offset..].find(['?', '#']) {
        Some(b) => offset + b, None => input.len()
    };
    Ok(path_end)
}

fn parse_query(input: &str, offset: usize) -> Result<usize, ()> {
    if offset >= input.len() { return Ok(offset); }

    let query_end = match input[offset..].find('#') {
        Some(b) => offset + b, None => input.len()
    };
    Ok(query_end)
}

impl Url<'_> {
    pub fn scheme(&self) -> &str {
        &self.url[..self.scheme_end]
    }
    pub fn authority(&self) -> &str {
        if matches!(self.scheme(), "http" | "https" | "ws" | "wss" | "ftp" | "file") {
            &self.url[self.scheme_end+3..self.authority_end] // account for "://"
        } else {
            &self.url[self.scheme_end+1..self.authority_end] // account for ":"
        }
    }
    pub fn host(&self) -> &str {
        &self.url[self.userinfo_end..self.host_end]
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub fn path_and_query(&self) -> &str {
        &self.url[self.authority_end..self.query_end]
    }
    pub fn path(&self) -> &str {
        // leading slashes are ok
        &self.url[self.authority_end..self.path_end]
    }
    pub fn query(&self) -> &str {
        if self.path_end < self.len() {
            &self.url[self.path_end+1..self.query_end] // account for '?'
        } else {
            &self.url[self.path_end..self.query_end]
        }
    }
    pub fn fragment(&self) -> &str {
        if self.path_end < self.len() {
            &self.url[self.query_end+1..] // account for '#'
        } else {
            &self.url[self.query_end..]
        }
    }
    pub const fn len(&self) -> usize { self.url.len() }
    pub const fn is_empty(&self) -> bool { self.url.is_empty() }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    #[test]
    fn basic_parse_url() {
        let parsed_url = parse_url("http://www.example.org").unwrap();

        assert_eq!("http", parsed_url.scheme());
        assert_eq!("www.example.org", parsed_url.authority());
    }
    #[test]
    fn path_parse_url() {
        let parsed_url = parse_url("http://www.example.org/scripts/").unwrap();
        std::println!("url_len: {}", parsed_url.len());

        assert_eq!("http", parsed_url.scheme());
        assert_eq!("www.example.org", parsed_url.authority());
        assert_eq!("/scripts/", parsed_url.path_and_query());
    }
    #[test]
    fn path_and_query_parse_url() {
        let parsed_url = parse_url("http://www.example.org/scripts/?job=111&task=1").unwrap();

        assert_eq!("http", parsed_url.scheme());
        assert_eq!("www.example.org", parsed_url.authority());
        assert_eq!("/scripts/?job=111&task=1", parsed_url.path_and_query());
        assert_eq!("job=111&task=1", parsed_url.query());
    }
    #[test]
    fn single_slash_path_parse_url() {
        let parsed_url = parse_url("http://www.example.org/").unwrap();

        assert_eq!("www.example.org", parsed_url.authority());
        assert_eq!("/", parsed_url.path());
    }
    #[test]
    fn fragment_parse_url() {
        let parsed_url = parse_url("http://www.example.org/#introduction").unwrap();

        assert_eq!("/", parsed_url.path_and_query());
        assert_eq!("introduction", parsed_url.fragment());
    }
    #[test]
    fn port_parse_url() {
        let parsed_url = parse_url("http://www.example.org:8080").unwrap();

        assert_eq!("www.example.org", parsed_url.host());
        assert_eq!(8080, parsed_url.port());
    }
}
