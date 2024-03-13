# Squid Roadmap

## 0.1.0
- [X] Url struct
    - [X] impl FromStr
    - [X] (de)serializable
    - [ ] print out indiviudal components
- [X] parsing functions

## Features

## API
- [X] master Url struct
    - contains every section of the url
- [X] parsing
    - parse from &str into Url (FromStr)
    - each url components parses separately
    - [ ] parse individual components into Url struct
    - [ ] support optional base Url

## Long-term goals
- [ ] minimal allocations
- [ ] minimal dependencies
- [ ] no alloc crate
