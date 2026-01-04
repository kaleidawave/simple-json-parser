# [JSON](https://www.json.org/json-en.html) parser/lexer

![lines of code](https://kaleidawave-projectinformation.web.val.run/project/simple-json-parser/badge)
[![crates.io badge](https://img.shields.io/crates/v/simple-json-parser?style=flat-square)](https://crates.io/crates/simple-json-parser)
[![docs.rs badge](https://img.shields.io/docsrs/simple-json-parser?style=flat-square)](https://docs.rs/simple-json-parser/latest)

Features
- Under < 300 LOC Rust lexer
- No dependencies
- Visiting / callback based API (avoids allocations)
- Handles single (both `#` and `//`) and multiline comments (`/*`) in JSON
- Supports partials (aka values ended early by `,`, `]` or `}`)
- New line delimeted parsing

See [examples](/examples/) and [tests](/tests/) for usage.

### TODO

- number parsing?
- string and number wrappers
- test for partial parsing
- move the to-object file, as a module under `feature=extra`

> do all examples work? should some of the implementations be under cfg? maybe extras
