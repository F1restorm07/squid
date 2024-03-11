# Squid Roadmap

## 0.1.0
- [ ] URL struct
    - [ ] impl FromStr
    - [ ] support special URL schemes
- [ ] parsing functions

## Features

## API
- [ ] master URL struct
    - contains every section of the url
- [ ] parsing
    - parse from &str into Url (FromStr)
    - each url components parses separately, returning the parsed input and the rest of the input as output
        - Ex: fn(&str) -> (<host>, &str)

## Long-term goals
- [ ] minimal allocations
- [ ] minimal dependencies
