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
const WHITESPACE: [char; 4] = [' ', '\t', '\r', '\n'];

enum State {
    InKey {
        escaped: bool,
        start: usize,
    },
    Colon,
    InObject {
        last_was_comma: bool,
        /// For [`RootJSONValue::Object`]
        found: bool,
    },
    Comment {
        start: usize,
        multiline: bool,
        /// For deciding whether at end of a multiline comment
        last_was_asterisk: bool,
        /// Whether single-line *hash* style comments: `# comment`
        hash: bool,
    },
    ExpectingValue {
        /// For [`RootJSONValue::Array`]
        found: bool,
    },
    StringValue {
        start: usize,
        escaped: bool,
    },
    NumberValue {
        start: usize,
        // Whether seen the '.'
        fractional: bool,
        // Whether seen the 'e'
        exponent: bool,
    },
    TrueFalseNull {
        start: usize,
    },
    EndOfValue,
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
    let chars = on.char_indices();

    let mut key_chain = Vec::new();

    if options.top_level_separator.is_some() {
        key_chain.push(JSONKey::Index(0));
    }

    let mut state = State::ExpectingValue { found: true };

    for (idx, chr) in chars {
        match state {
            State::InKey {
                start,
                ref mut escaped,
            } => {
                if !*escaped && chr == '"' {
                    key_chain.push(JSONKey::Slice(&on[start..idx]));
                    state = State::Colon;
                } else {
                    *escaped = chr == '\\';
                }
            }
            State::StringValue {
                start,
                ref mut escaped,
            } => {
                if !*escaped && chr == '"' {
                    state = State::EndOfValue;
                    let res = cb(&key_chain, RootJSONValue::String(&on[start..idx]));
                    if res.is_some() {
                        return Ok((idx + chr.len_utf8(), res));
                    }
                } else if *escaped {
                    *escaped = false;
                } else {
                    *escaped = chr == '\\';
                }
            }
            State::Colon => {
                if chr == ':' {
                    state = State::ExpectingValue { found: true };
                } else if !WHITESPACE.contains(&chr) {
                    return Err(JSONParseError {
                        at: idx,
                        reason: JSONParseErrorReason::ExpectedColon,
                    });
                }
            }
            State::EndOfValue => {
                end_of_value(idx, chr, &mut state, &mut key_chain, options.allow_comments)?;

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
                // else if key_chain.is_empty() {
                //     // TODO check rest is empty
                //     return Ok((idx + chr.len_utf8(), None));
                // }
            }
            State::Comment {
                ref mut last_was_asterisk,
                ref mut multiline,
                hash,
                start,
            } => {
                let at_end = (chr == '\n' && !*multiline)
                    || (*multiline && *last_was_asterisk && chr == '/');
                if at_end {
                    if options.yield_comments {
                        let comment = if *multiline {
                            &on[start..=idx]
                        } else {
                            &on[start..idx]
                        };
                        let res = cb(&key_chain, RootJSONValue::Comment(comment));
                        if res.is_some() {
                            return Ok((idx, res));
                        }
                    }
                    if let Some(JSONKey::Index(..)) = key_chain.last() {
                        state = State::ExpectingValue { found: true };
                    } else {
                        state = State::InObject {
                            last_was_comma: false,
                            found: true,
                        };
                    }
                } else if chr == '*' && start + 1 == idx && !hash {
                    *multiline = true;
                } else if *multiline {
                    *last_was_asterisk = chr == '*';
                }
            }
            State::ExpectingValue { found } => {
                state = match chr {
                    '[' => {
                        key_chain.push(JSONKey::Index(0));
                        State::ExpectingValue { found: false }
                    }
                    ']' => {
                        key_chain.pop();
                        if !found {
                            cb(&key_chain, RootJSONValue::EmptyArray);
                        }
                        State::EndOfValue
                    }
                    '{' => State::InObject {
                        last_was_comma: false,
                        found: false,
                    },
                    // '}' => {
                    //     if let Some(JSONKey::Index(..)) = key_chain.pop() {
                    //         State::ExpectingValue
                    //     } else {
                    //         dbg!();
                    //         // TODO should error
                    //         State::InObject {
                    //             last_was_comma: false,
                    //         }
                    //     }
                    // }
                    '"' => State::StringValue {
                        start: idx + '"'.len_utf8(),
                        escaped: false,
                    },
                    c @ ('/' | '#') if options.allow_comments => State::Comment {
                        last_was_asterisk: false,
                        start: idx,
                        multiline: false,
                        hash: c == '#',
                    },
                    '0'..='9' | '-' => State::NumberValue {
                        start: idx,
                        fractional: false,
                        exponent: false,
                    },
                    't' | 'f' | 'n' => State::TrueFalseNull { start: idx },
                    ',' | '}' if options.partial_syntax => {
                        let res = cb(&key_chain, RootJSONValue::Empty);
                        if res.is_some() {
                            return Ok((idx, res));
                        }
                        // let x = key_chain.pop();
                        let mut state = state;
                        end_of_value(idx, chr, &mut state, &mut key_chain, options.allow_comments)?;
                        state
                    }
                    chr if WHITESPACE.contains(&chr) => state,
                    _ => {
                        return Err(JSONParseError {
                            at: idx,
                            reason: JSONParseErrorReason::ExpectedValue,
                        });
                    }
                }
            }
            State::InObject { last_was_comma, found } => {
                if chr == '"' {
                    state = State::InKey {
                        escaped: false,
                        start: idx + '"'.len_utf8(),
                    };
                } else if chr == '}' {
                    if !found {
                        cb(&key_chain, RootJSONValue::EmptyObject);
                    }
                    if last_was_comma && !options.allow_trailing_commas {
                        return Err(JSONParseError {
                            at: idx,
                            reason: JSONParseErrorReason::ExpectedKey,
                        });
                    }
                    state = State::EndOfValue;
                } else if let (true, c @ ('/' | '#')) = (options.allow_comments, chr) {
                    state = State::Comment {
                        last_was_asterisk: false,
                        start: idx,
                        multiline: false,
                        hash: c == '#',
                    };
                } else if !WHITESPACE.contains(&chr) {
                    return Err(JSONParseError {
                        at: idx,
                        reason: JSONParseErrorReason::ExpectedKey,
                    });
                }
            }
            State::NumberValue {
                start,
                ref mut fractional,
                ref mut exponent,
            } => {
                if let '0' | '1'..='9' = chr {
                    // TODO different handling for 0
                } else if let '.' = chr {
                    if *fractional {
                        todo!("error")
                    }
                    if *exponent {
                        todo!("error")
                    }
                    *fractional = true;
                } else if let '-' | '+' = chr {
                    todo!("check last was exponent")
                } else if let 'e' | 'E' = chr {
                    if *fractional {
                        todo!("error")
                    }
                    if *exponent {
                        todo!("error")
                    }
                    *exponent = true;
                } else {
                    // I think this is fine
                    let res = cb(&key_chain, RootJSONValue::Number(&on[start..idx]));
                    if res.is_some() {
                        return Ok((idx, res));
                    }
                    state = State::EndOfValue;
                    end_of_value(idx, chr, &mut state, &mut key_chain, options.allow_comments)?;
                }
            }
            State::TrueFalseNull { start } => {
                let diff = idx - start + 1;
                if diff < 4 {
                    // ...
                } else if diff == 4 {
                    match &on[start..(idx + chr.len_utf8())] {
                        "true" => {
                            let res = cb(&key_chain, RootJSONValue::Boolean(true));
                            if res.is_some() {
                                return Ok((idx + chr.len_utf8(), res));
                            }
                            state = State::EndOfValue;
                        }
                        "null" => {
                            let res = cb(&key_chain, RootJSONValue::Null);
                            if res.is_some() {
                                return Ok((idx + chr.len_utf8(), res));
                            }
                            state = State::EndOfValue;
                        }
                        "fals" => {}
                        _ => {
                            return Err(JSONParseError {
                                at: idx,
                                reason: JSONParseErrorReason::ExpectedTrueFalseNull,
                            })
                        }
                    }
                } else if let "false" = &on[start..(idx + chr.len_utf8())] {
                    let res = cb(&key_chain, RootJSONValue::Boolean(false));
                    if res.is_some() {
                        return Ok((idx + chr.len_utf8(), res));
                    }
                    state = State::EndOfValue;
                } else {
                    return Err(JSONParseError {
                        at: idx,
                        reason: JSONParseErrorReason::ExpectedTrueFalseNull,
                    });
                }
            }
        }
    }

    match state {
        State::InKey { .. } | State::StringValue { .. } => {
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
        State::Comment { multiline, .. } => {
            if multiline {
                return Err(JSONParseError {
                    at: on.len(),
                    reason: JSONParseErrorReason::ExpectedEndOfMultilineComment,
                });
            }
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
        State::NumberValue { start, .. } => {
            // TODO actual number handing
            let _result = cb(&key_chain, RootJSONValue::Number(&on[start..]));
        }
        State::TrueFalseNull { start: _ } => {
            return Err(JSONParseError {
                at: on.len(),
                reason: JSONParseErrorReason::ExpectedTrueFalseNull,
            })
        }
    }

    Ok((on.len(), None))
}

// TODO always pops from key_chain **unless** we are in an array.
// TODO there are complications using this in an iterator when we yielding numbers
fn end_of_value(
    idx: usize,
    chr: char,
    state: &mut State,
    key_chain: &mut Vec<JSONKey<'_>>,
    allow_comments: bool,
) -> Result<(), JSONParseError> {
    if chr == ',' {
        if let Some(JSONKey::Index(i)) = key_chain.last_mut() {
            *i += 1;
            *state = State::ExpectingValue { found: true };
        } else {
            key_chain.pop();
            *state = State::InObject {
                last_was_comma: true,
                found: true,
            };
        }
    } else if let ('}', Some(JSONKey::Slice(..))) = (chr, key_chain.last()) {
        // TODO errors here if index
        key_chain.pop();
    } else if let (']', Some(JSONKey::Index(..))) = (chr, key_chain.last()) {
        // TODO errors here if slice etc
        key_chain.pop();
    } else if let (true, c @ ('/' | '#')) = (allow_comments, chr) {
        key_chain.pop();
        *state = State::Comment {
            last_was_asterisk: false,
            start: idx,
            multiline: false,
            hash: c == '#',
        };
    } else if !WHITESPACE.contains(&chr) {
        return Err(JSONParseError {
            at: idx,
            reason: JSONParseErrorReason::ExpectedEndOfValue,
        });
    }
    Ok(())
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
        result += &on[start..index];
        match on[index + 1..].chars().next() {
            Some('"' | '\'') => {}
            Some('t') => {
                result += "\t";
            }
            Some('n') => {
                result += "\n";
            }
            Some('r') => {
                result += "\r";
            }
            Some(chr) => {
                eprintln!("unexpected item {chr:?}");
            }
            None => {
                eprintln!("end of item?");
            }
        }
        start = index + 1;
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
    fn unescaping() {
        assert!(matches!(
            unescape_string_content("No quotes here"),
            Cow::Borrowed(_)
        ));
        assert_eq!(
            unescape_string_content("Something with \\\"quotes\\\""),
            "Something with \"quotes\""
        );
    }
}
