# JSON parser/lexer

![lines of code](https://kaleidawave-projectinformation.web.val.run/project/simple-json-parser/badge)
[![crates.io badge](https://img.shields.io/crates/v/simple-json-parser?style=flat-square)](https://crates.io/crates/simple-json-parser)
[![docs.rs badge](https://img.shields.io/docsrs/simple-json-parser?style=flat-square)](https://docs.rs/simple-json-parser/latest)

Features
- Under < 200 LOC Rust lexer
- No dependencies
- Visiting / callback based API (avoids allocations)
- Handles single and multiline comments in JSON

See [examples](/examples/) and [tests](/tests/) for usage.

### TODO

> is there a change to end_of_value on laptop?

- end_of_value: if key chain len 1 and character == new_line_delimeter => State::ExpectingValue
- end_of_value: do not drop last in array key chain if new_line_delimeter is some
- parse should handle more flags
- finish number parsing (lots of `todo`s)
- test partials

> do all examples work? should some of the implementations be under cfg? maybe extras
