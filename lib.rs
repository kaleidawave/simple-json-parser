#![doc = include_str!("./README.md")]

use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum JSONKey<'a> {
    Slice(&'a str),
    Index(usize),
}

#[derive(Debug, PartialEq, Eq)]
pub enum RootJSONValue<'a> {
    String(&'a str),
    Number(&'a str),
    Boolean(bool),
    Null,
    EmptyObject,
    EmptyArray,
    /// Under `yield_comments` these are *sometimes* emitted to preserve formatting
    Comment(&'a str),
    /// For `options.partial_syntax` WIP
    Empty,
}

#[derive(Debug)]
pub enum JSONParseErrorReason {
    ExpectedColon,
    ExpectedEndOfValue,
    ExpectedEndOfKey,
    /// Doubles as both closing and ending
    ExpectedBracket,
    ExpectedTrueFalseNull,
    ExpectedKey,
    ExpectedValue,
    ExpectedEndOfMultilineComment,
    /// Both for string values and keys
    ExpectedQuote,
}

#[derive(Debug)]
pub struct JSONParseError {
    pub at: usize,
    pub reason: JSONParseErrorReason,
}

impl std::error::Error for JSONParseError {}

impl std::fmt::Display for JSONParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_fmt(format_args!(
            "JSONParseError: {:?} at {:?}",
            self.reason, self.at
        ))
    }
}

/// If you want to return early (break on an exception in the callback) or
/// more configuration use [`parse_with_options`]
///
/// # Errors
/// Returns an error if it tries to parse invalid JSON input
pub fn parse<'a>(
    on: &'a str,
    mut cb: impl for<'b> FnMut(&'b [JSONKey<'a>], RootJSONValue<'a>),
) -> Result<usize, JSONParseError> {
    let options = ParseOptions::default();
    parse_with_options(on, &options, |k, v| {
        cb(k, v);
        None::<()>
    })
    .map(|(parsed, _)| parsed)
}

#[derive(Default, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct ParseOptions {
    pub allow_trailing_commas: bool,
    pub partial_syntax: bool,
    pub allow_comments: bool,
    // TODO combine with above
    pub yield_comments: bool,
    // For new line JSON etc
    pub top_level_separator: Option<&'static str>,
}

/// The four characters considered by the JSON specification as *whitespace*
// const WHITESPACE: [char; 4] = [' ', '\t', '\r', '\n'];
const WHITESPACE: [u8; 4] = [b' ', b'\t', b'\r', b'\n'];

enum State {
    Key,
    Colon,
    InObject {
        last_was_comma: bool,
        /// For [`RootJSONValue::Object`]
        found: bool,
    },
    Comment,
    ExpectingValue {
        /// For [`RootJSONValue::Array`]
        found: bool,
    },
    EndOfValue,
}

#[allow(clippy::manual_find)]
fn find_non_escaped(on: &str) -> Option<usize> {
    for (idx, _) in on.match_indices('"') {
        if !on[..idx].ends_with('\\') {
            return Some(idx);
        }
    }
    None
}

/// Returns the number of bytes parsed.
/// `exit_on_first_value` returns once the first object has been parsed.
///
/// # Errors
/// Returns an error if it tries to parse invalid JSON input
#[allow(clippy::too_many_lines)]
pub fn parse_with_options<'a, T>(
    on: &'a str,
    options: &ParseOptions,
    mut cb: impl for<'b> FnMut(&'b [JSONKey<'a>], RootJSONValue<'a>) -> Option<T>,
) -> Result<(usize, Option<T>), JSONParseError> {
    let mut key_chain = Vec::new();

    if options.top_level_separator.is_some() {
        key_chain.push(JSONKey::Index(0));
    }

    let mut state = State::ExpectingValue { found: true };

    let mut idx = 0;
    while idx < on.len() {
        match state {
            State::Key => {
                let rest = &on[idx..];
                let Some(offset) = find_non_escaped(rest) else {
                    return Err(JSONParseError {
                        at: idx,
                        reason: JSONParseErrorReason::ExpectedEndOfKey,
                    });
                };
                key_chain.push(JSONKey::Slice(&rest[..offset]));
                state = State::Colon;
                idx += offset + 1;
            }
            State::Colon => {
                let slice = on.as_bytes();
                let chr = slice[idx];
                if chr == b':' {
                    state = State::ExpectingValue { found: true };
                    idx += 1;
                } else if WHITESPACE.contains(&chr) {
                    idx += 1;
                } else {
                    // TODO partial could find next ':'?
                    return Err(JSONParseError {
                        at: idx,
                        reason: JSONParseErrorReason::ExpectedColon,
                    });
                }
            }
            State::EndOfValue => {
                let slice = on.as_bytes();
                let byte = slice[idx];
                match byte {
                    b',' => {
                        idx += 1;
                        if let Some(JSONKey::Index(i)) = key_chain.last_mut() {
                            *i += 1;
                            state = State::ExpectingValue { found: true };
                        } else {
                            key_chain.pop();
                            state = State::InObject {
                                last_was_comma: true,
                                found: true,
                            };
                        }
                    }
                    b'}' if matches!(key_chain.last(), Some(JSONKey::Slice(_))) => {
                        idx += 1;
                        key_chain.pop();
                    }
                    b']' if matches!(key_chain.last(), Some(JSONKey::Index(_))) => {
                        idx += 1;
                        key_chain.pop();
                    }
                    b'/' | b'#' => {
                        state = State::Comment;
                    }
                    byte if WHITESPACE.contains(&byte) => {
                        idx += 1;
                    }
                    _ => {
                        dbg!(&on[idx..]);
                        return Err(JSONParseError {
                            at: idx,
                            reason: JSONParseErrorReason::ExpectedEndOfValue,
                        });
                    }
                }

                if let Some(separator) = options.top_level_separator {
                    if on[idx..].starts_with(separator) {
                        if let [JSONKey::Index(ref mut idx)] = key_chain[..] {
                            *idx += 1;
                            state = State::ExpectingValue { found: true };
                        } else {
                            // this is fine if we have not reached end
                            // unreachable!("{key_chain:?} should be empty")
                        }
                    } else {
                        // TODO can be error
                    }
                }
            }
            State::Comment => {
                let rest = &on[idx..];
                if rest.starts_with('#') {
                    let offset = rest.find('\n').unwrap_or(rest.len());
                    idx += offset;
                    if options.yield_comments {
                        let res = cb(&key_chain, RootJSONValue::Comment(&rest[..offset][1..]));
                        if res.is_some() {
                            return Ok((idx, res));
                        }
                    }
                } else if rest.starts_with("//") {
                    let offset = rest.find('\n').unwrap_or(rest.len());
                    idx += offset;
                    if options.yield_comments {
                        let res = cb(&key_chain, RootJSONValue::Comment(&rest[..offset][2..]));
                        if res.is_some() {
                            return Ok((idx, res));
                        }
                    }
                } else if rest.starts_with("*/") {
                    let Some(offset) = rest.find("*/") else {
                        return Err(JSONParseError {
                            at: idx,
                            reason: JSONParseErrorReason::ExpectedEndOfMultilineComment,
                        });
                    };
                    idx += offset + 2;
                    if options.yield_comments {
                        let res = cb(&key_chain, RootJSONValue::Comment(&rest[..offset][2..]));
                        if res.is_some() {
                            return Ok((idx, res));
                        }
                    }
                } else {
                    return Err(JSONParseError {
                        at: idx,
                        reason: JSONParseErrorReason::ExpectedKey,
                    });
                }
                if let Some(JSONKey::Index(..)) = key_chain.last() {
                    state = State::ExpectingValue { found: true };
                } else {
                    state = State::InObject {
                        last_was_comma: false,
                        found: true,
                    };
                }
            }
            State::ExpectingValue { found } => {
                let slice = on.as_bytes();
                let byte = slice[idx];
                state = match byte {
                    b'[' => {
                        idx += 1;
                        key_chain.push(JSONKey::Index(0));
                        State::ExpectingValue { found: false }
                    }
                    b']' => {
                        idx += 1;
                        key_chain.pop();
                        if !found {
                            cb(&key_chain, RootJSONValue::EmptyArray);
                        }
                        State::EndOfValue
                    }
                    b'{' => {
                        idx += 1;
                        State::InObject {
                            last_was_comma: false,
                            found: false,
                        }
                    }
                    b'/' | b'#' if options.allow_comments => State::Comment,
                    b'"' => {
                        idx += 1;
                        let rest = &on[idx..];
                        let Some(offset) = find_non_escaped(rest) else {
                            return Err(JSONParseError {
                                at: idx,
                                reason: JSONParseErrorReason::ExpectedEndOfValue,
                            });
                        };
                        let res = cb(&key_chain, RootJSONValue::String(&rest[..offset]));
                        idx += offset + 1;
                        if res.is_some() {
                            return Ok((idx, res));
                        }
                        State::EndOfValue
                    }
                    b'0'..=b'9' | b'-' => {
                        fn find_non_number(chr: char) -> bool {
                            !matches!(chr, '0'..='9' | '.' | 'e' | 'E' | '+' | '-')
                        }
                        // TODO delegate somewhere
                        let rest = &on[idx..];

                        let Some(offset) = rest.find(find_non_number) else {
                            return Err(JSONParseError {
                                at: idx,
                                reason: JSONParseErrorReason::ExpectedEndOfValue,
                            });
                        };
                        idx += offset;

                        // I think this is fine
                        let res = cb(&key_chain, RootJSONValue::Number(&rest[..offset]));
                        if res.is_some() {
                            return Ok((idx, res));
                        }
                        State::EndOfValue
                    }
                    b't' if on[idx..].starts_with("true") => {
                        idx += 4;
                        let res = cb(&key_chain, RootJSONValue::Boolean(true));
                        if res.is_some() {
                            return Ok((idx, res));
                        }
                        State::EndOfValue
                    }
                    b'f' if on[idx..].starts_with("false") => {
                        idx += 5;
                        let res = cb(&key_chain, RootJSONValue::Boolean(false));
                        if res.is_some() {
                            return Ok((idx, res));
                        }
                        State::EndOfValue
                    }
                    b'n' if on[idx..].starts_with("null") => {
                        idx += 4;
                        let res = cb(&key_chain, RootJSONValue::Null);
                        if res.is_some() {
                            return Ok((idx, res));
                        }
                        State::EndOfValue
                    }
                    b',' | b'}' if options.partial_syntax => {
                        idx += 1;
                        let res = cb(&key_chain, RootJSONValue::Empty);
                        let _ = key_chain.pop();
                        if res.is_some() {
                            return Ok((idx, res));
                        }
                        State::InObject {
                            last_was_comma: byte == b',',
                            found: false,
                        }
                    }
                    byte if WHITESPACE.contains(&byte) => {
                        idx += 1;
                        state
                    }
                    _ => {
                        return Err(JSONParseError {
                            at: idx,
                            reason: JSONParseErrorReason::ExpectedValue,
                        });
                    }
                }
            }
            State::InObject {
                last_was_comma,
                found,
            } => {
                let slice = on.as_bytes();
                let byte = slice[idx];
                match byte {
                    b'"' => {
                        idx += 1;
                        state = State::Key;
                    }
                    b'}' => {
                        if last_was_comma && !options.allow_trailing_commas {
                            return Err(JSONParseError {
                                at: idx,
                                reason: JSONParseErrorReason::ExpectedKey,
                            });
                        }
                        idx += 1;
                        if !found {
                            cb(&key_chain, RootJSONValue::EmptyObject);
                        }
                        state = State::EndOfValue;
                    }
                    b'/' | b'#' if options.allow_comments => {
                        state = State::Comment;
                    }
                    byte if WHITESPACE.contains(&byte) => {
                        idx += 1;
                    }
                    _ => {
                        dbg!(&on[idx - 1..]);
                        dbg!(&on[idx..]);
                        return Err(JSONParseError {
                            at: idx,
                            reason: JSONParseErrorReason::ExpectedKey,
                        });
                    }
                }
            }
        }
    }

    match state {
        State::Key => {
            return Err(JSONParseError {
                at: on.len(),
                reason: JSONParseErrorReason::ExpectedQuote,
            })
        }
        State::Colon => {
            return Err(JSONParseError {
                at: on.len(),
                reason: JSONParseErrorReason::ExpectedColon,
            });
        }
        State::Comment => {
            return Err(JSONParseError {
                at: on.len(),
                reason: JSONParseErrorReason::ExpectedEndOfMultilineComment,
            });
        }
        State::EndOfValue | State::ExpectingValue { found: _ } => {
            let okay = match options.top_level_separator {
                Some(_) => matches!(&key_chain[..], [JSONKey::Index(_)]),
                None => key_chain.is_empty(),
            };
            if !okay {
                return Err(JSONParseError {
                    at: on.len(),
                    // TODO might be different based on `options.top_level_separator`
                    reason: JSONParseErrorReason::ExpectedBracket,
                });
            }
        }
        State::InObject { .. } => {
            return Err(JSONParseError {
                at: on.len(),
                reason: JSONParseErrorReason::ExpectedBracket,
            });
        }
    }

    Ok((on.len(), None))
}

/// Equates key chains while accounting for escapes
#[must_use]
pub fn key_chain_equals(keys: &[JSONKey<'_>], expected: &[JSONKey<'_>]) -> bool {
    if keys.len() == expected.len() {
        for (expected, key) in std::iter::zip(expected, keys) {
            match (expected, key) {
                (JSONKey::Slice(expected), JSONKey::Slice(key)) => {
                    let mut key_chars = key.chars();
                    for expected in expected.chars() {
                        let next = key_chars.next();
                        // Extract escapes
                        let next = if next.is_some_and(|inner| inner == '\\') {
                            key_chars.next()
                        } else {
                            next
                        };
                        if next.is_none_or(|key_chr| expected != key_chr) {
                            return false;
                        }
                    }
                }
                (JSONKey::Index(expected), JSONKey::Index(key)) => {
                    if expected != key {
                        return false;
                    }
                }
                (_, _) => return false,
            }
        }
        true
    } else {
        false
    }
}

/// Modified version of <https://github.com/parcel-bundler/parcel/blob/f86f5f27c3a6553e70bd35652f19e6ab8d8e4e4a/crates/dev-dep-resolver/src/lib.rs#L368-L380>
#[must_use]
pub fn unescape_string_content(on: &str) -> Cow<'_, str> {
    let mut result = Cow::Borrowed("");
    let mut start = 0;
    for (index, _matched) in on.match_indices('\\') {
        if index <= start {
            continue;
        }
        result += &on[start..index];
        match on[index..][1..].chars().next() {
            Some('"' | '\\' | '/') => {
                start = index + 1;
            }
            Some('b') => {
                // backspace
                result += "\u{08}";
                start = index + 2;
            }
            Some('f') => {
                // formfeed
                result += "\u{0c}";
                start = index + 2;
            }
            Some('n') => {
                result += "\n";
                start = index + 2;
            }
            Some('r') => {
                result += "\r";
                start = index + 2;
            }
            Some('t') => {
                result += "\t";
                start = index + 2;
            }
            Some('u') => {
                fn parse_hex(on: &str) -> Result<u32, &str> {
                    let mut value = 0u32;
                    for byte in on.bytes() {
                        value <<= 4; // log2(16) = 4
                        let code = match byte {
                            b'0'..=b'9' => u32::from(byte - b'0'),
                            b'a'..=b'f' => u32::from(byte - b'a') + 10,
                            b'A'..=b'F' => u32::from(byte - b'A') + 10,
                            _byte => {
                                return Err(on);
                            }
                        };
                        value |= code;
                    }
                    Ok(value)
                }

                let unicode_char = on[index..][2..]
                    .get(0..6)
                    .and_then(|s| s.strip_prefix('{'))
                    .and_then(|s| s.strip_suffix('}'))
                    .and_then(|slice| parse_hex(slice).ok())
                    .and_then(char::from_u32);

                if let Some(item) = unicode_char {
                    result.to_mut().push(dbg!(item));
                    start = index + 8;
                } else {
                    start = index;
                    eprintln!("expected 4 hex digits");
                }
            }
            Some(chr) => {
                start = index + 1;
                eprintln!("unexpected item {chr:?}");
            }
            // This is unreachable with the results returned
            // from JSON parsing
            None => {
                start = index;
                eprintln!("end of item?");
            }
        }
    }
    result += &on[start..];
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_chain_equality() {
        assert!(key_chain_equals(
            &[JSONKey::Slice("k1"), JSONKey::Slice("q\\\"")],
            &[JSONKey::Slice("k1"), JSONKey::Slice("q\"")]
        ));
        assert!(!key_chain_equals(
            &[JSONKey::Slice("k1"), JSONKey::Slice("b\\\"")],
            &[JSONKey::Slice("k1"), JSONKey::Slice("q\"")]
        ));
    }

    #[test]
    fn unescaping_none_no_transform() {
        // We do no allocate when transformation is not done
        // assert!(unescape_string_content("No quotes here").is_borrowed());
        assert!(matches!(unescape_string_content("No quotes here"), Cow::Borrowed(_)));
    }
    
    #[test]
    fn unescaping() {
        assert_eq!(
            unescape_string_content("Something with \\\"quotes\\\""),
            "Something with \"quotes\""
        );
        assert_eq!(
            unescape_string_content("tab\\t and newline\n"),
            "tab\t and newline\n"
        );
        assert_eq!(unescape_string_content("hex\\u{0021}"), "hex!");
    }

    #[test]
    fn unescaping_unknown_or_invalid() {
        assert_eq!(unescape_string_content("not \\an escape"), "not an escape");
        assert_eq!(unescape_string_content("not \\u{34} escape"), "not \\u{34} escape");
    }

    #[test]
    fn unescaping_end() {
        assert_eq!(unescape_string_content("ends with \\"), "ends with \\");
    }
}
