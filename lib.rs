use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq)]
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
/// more configuration use [`parse_with_exit_signal`]
///
/// # Errors
/// Returns an error if it tries to parse invalid JSON input
pub fn parse<'a>(
    on: &'a str,
    mut cb: impl for<'b> FnMut(&'b [JSONKey<'a>], RootJSONValue<'a>),
) -> Result<usize, JSONParseError> {
    parse_with_exit_signal(
        on,
        |k, v| {
            cb(k, v);
            false
        },
        &ParseOptions::default(),
    )
}

enum State {
    InKey {
        escaped: bool,
        start: usize,
    },
    Colon,
    InObject,
    Comment {
        start: usize,
        multiline: bool,
        last_was_asterisk: bool,
        hash: bool,
    },
    ExpectingValue,
    StringValue {
        start: usize,
        escaped: bool,
    },
    NumberValue {
        start: usize,
    },
    TrueFalseNull {
        start: usize,
    },
    EndOfValue,
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
            *state = State::ExpectingValue;
        } else {
            key_chain.pop();
            *state = State::InObject;
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
    } else if !chr.is_whitespace() {
        return Err(JSONParseError {
            at: idx,
            reason: JSONParseErrorReason::ExpectedEndOfValue,
        });
    }
    Ok(())
}

#[derive(Default)]
pub struct ParseOptions {
    pub exit_on_first_value: bool,
    pub allow_comments: bool,
}

/// Returns the number of bytes parsed.
/// `exit_on_first_value` returns once the first object has been parsed.
///
/// # Errors
/// Returns an error if it tries to parse invalid JSON input
#[allow(clippy::too_many_lines)]
pub fn parse_with_exit_signal<'a>(
    on: &'a str,
    mut cb: impl for<'b> FnMut(&'b [JSONKey<'a>], RootJSONValue<'a>) -> bool,
    options: &ParseOptions,
) -> Result<usize, JSONParseError> {
    let chars = on.char_indices();

    let mut key_chain = Vec::new();
    let mut state = State::ExpectingValue;

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
                    if res {
                        return Ok(idx + chr.len_utf8());
                    }
                } else if *escaped {
                    *escaped = false;
                } else {
                    *escaped = chr == '\\';
                }
            }
            State::Colon => {
                if chr == ':' {
                    state = State::ExpectingValue;
                } else if !chr.is_whitespace() {
                    return Err(JSONParseError {
                        at: idx,
                        reason: JSONParseErrorReason::ExpectedColon,
                    });
                }
            }
            State::EndOfValue => {
                end_of_value(idx, chr, &mut state, &mut key_chain, options.allow_comments)?;

                if options.exit_on_first_value && key_chain.is_empty() && chr != ',' {
                    return Ok(idx + chr.len_utf8());
                }
            }
            State::Comment {
                ref mut last_was_asterisk,
                ref mut multiline,
                hash,
                start,
            } => {
                if chr == '\n' && !*multiline {
                    if let Some(JSONKey::Index(..)) = key_chain.last() {
                        state = State::ExpectingValue;
                    } else {
                        state = State::InObject;
                    }
                } else if chr == '*' && start + 1 == idx && !hash {
                    *multiline = true;
                } else if *multiline {
                    if *last_was_asterisk && chr == '/' {
                        if let Some(JSONKey::Index(..)) = key_chain.last() {
                            state = State::ExpectingValue;
                        } else {
                            state = State::InObject;
                        }
                    } else {
                        *last_was_asterisk = chr == '*';
                    }
                }
            }
            State::ExpectingValue => {
                state = match chr {
                    '{' => State::InObject,
                    '[' => {
                        key_chain.push(JSONKey::Index(0));
                        State::ExpectingValue
                    }
                    ']' => {
                        key_chain.pop();
                        State::EndOfValue
                    }
                    '}' => {
                        if let Some(JSONKey::Index(..)) = key_chain.pop() {
                            State::ExpectingValue
                        } else {
                            State::InObject
                        }
                    }
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
                    '0'..='9' | '-' => State::NumberValue { start: idx },
                    't' | 'f' | 'n' => State::TrueFalseNull { start: idx },
                    chr if chr.is_whitespace() => state,
                    _ => {
                        return Err(JSONParseError {
                            at: idx,
                            reason: JSONParseErrorReason::ExpectedValue,
                        });
                    }
                }
            }
            State::InObject => {
                if chr == '"' {
                    state = State::InKey {
                        escaped: false,
                        start: idx + '"'.len_utf8(),
                    };
                } else if chr == '}' {
                    state = State::EndOfValue;
                } else if let (true, c @ ('/' | '#')) = (options.allow_comments, chr) {
                    state = State::Comment {
                        last_was_asterisk: false,
                        start: idx,
                        multiline: false,
                        hash: c == '#',
                    };
                } else if !chr.is_whitespace() {
                    return Err(JSONParseError {
                        at: idx,
                        reason: JSONParseErrorReason::ExpectedKey,
                    });
                }
            }
            State::NumberValue { start } => {
                // TODO actual number handing
                if chr.is_whitespace() || matches!(chr, '}' | ',' | ']') {
                    let res = cb(&key_chain, RootJSONValue::Number(&on[start..idx]));
                    if res {
                        return Ok(idx);
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
                            if res {
                                return Ok(idx + chr.len_utf8());
                            }
                            state = State::EndOfValue;
                        }
                        "null" => {
                            let res = cb(&key_chain, RootJSONValue::Null);
                            if res {
                                return Ok(idx + chr.len_utf8());
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
                    if res {
                        return Ok(idx + chr.len_utf8());
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
        State::EndOfValue | State::ExpectingValue => {
            if !key_chain.is_empty() {
                return Err(JSONParseError {
                    at: on.len(),
                    reason: JSONParseErrorReason::ExpectedBracket,
                });
            }
        }
        State::InObject => {
            return Err(JSONParseError {
                at: on.len(),
                reason: JSONParseErrorReason::ExpectedBracket,
            });
        }
        State::NumberValue { start } => {
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

    Ok(on.len())
}

// Equates key chains while accounting for escapes
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
