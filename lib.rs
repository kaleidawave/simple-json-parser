#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JSONKey<'a> {
    Slice(&'a str),
    Index(usize),
}

#[derive(Debug, PartialEq, Eq)]
pub enum RootJSONValue<'a> {
    String(&'a str),
    Number(&'a str),
    True,
    False,
    Null,
}

#[derive(Debug)]
pub enum JSONParseErrorReason {
    ExpectedColon,
    ExpectedEndOfValue,
    /// Doubles as both closing and ending
    ExpectedBracket,
    ExpectedTrueFalseNull,
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

/// If you want to return early (not parse the whole input) use [`parse_with_exit_signal`]
///
/// # Errors
/// Returns an error if it tries to parse invalid JSON input
pub fn parse<'a>(
    on: &'a str,
    mut cb: impl for<'b> FnMut(&'b [JSONKey<'a>], RootJSONValue<'a>),
) -> Result<(), JSONParseError> {
    parse_with_exit_signal(on, |k, v| {
        cb(k, v);
        false
    })
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
        key_chain.pop();
    } else if let (']', Some(JSONKey::Index(..))) = (chr, key_chain.last()) {
        key_chain.pop();
    } else if let c @ ('/' | '#') = chr {
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

/// # Errors
/// Returns an error if it tries to parse invalid JSON input
#[allow(clippy::too_many_lines)]
pub fn parse_with_exit_signal<'a>(
    on: &'a str,
    mut cb: impl for<'b> FnMut(&'b [JSONKey<'a>], RootJSONValue<'a>) -> bool,
) -> Result<(), JSONParseError> {
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
                        return Ok(());
                    }
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
                end_of_value(idx, chr, &mut state, &mut key_chain)?;
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
                    '"' => State::StringValue {
                        start: idx + '"'.len_utf8(),
                        escaped: false,
                    },
                    c @ ('/' | '#') => State::Comment {
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
                        })
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
                    if let Some(JSONKey::Index(..)) = key_chain.last() {
                        state = State::ExpectingValue;
                    } else {
                        state = State::InObject;
                    }
                }
            }
            State::NumberValue { start } => {
                // TODO actual number handing
                if chr.is_whitespace() || matches!(chr, '}' | ',' | ']') {
                    let res = cb(&key_chain, RootJSONValue::Number(&on[start..idx]));
                    if res {
                        return Ok(());
                    }
                    state = State::EndOfValue;
                    end_of_value(idx, chr, &mut state, &mut key_chain)?;
                }
            }
            State::TrueFalseNull { start } => {
                let diff = idx - start + 1;
                if diff < 4 {
                    // ...
                } else if diff == 4 {
                    match &on[start..=idx] {
                        "true" => {
                            let res = cb(&key_chain, RootJSONValue::True);
                            if res {
                                return Ok(());
                            }
                            state = State::EndOfValue;
                        }
                        "null" => {
                            let res = cb(&key_chain, RootJSONValue::Null);
                            if res {
                                return Ok(());
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
                } else if let "false" = &on[start..=idx] {
                    cb(&key_chain, RootJSONValue::False);
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
        State::InKey { .. } | State::StringValue { .. } => Err(JSONParseError {
            at: on.len(),
            reason: JSONParseErrorReason::ExpectedQuote,
        }),
        State::Colon => Err(JSONParseError {
            at: on.len(),
            reason: JSONParseErrorReason::ExpectedColon,
        }),
        State::Comment { multiline, .. } => {
            if multiline {
                Ok(())
            } else {
                Err(JSONParseError {
                    at: on.len(),
                    reason: JSONParseErrorReason::ExpectedEndOfMultilineComment,
                })
            }
        }
        State::EndOfValue | State::ExpectingValue => {
            if key_chain.is_empty() {
                Ok(())
            } else {
                Err(JSONParseError {
                    at: on.len(),
                    reason: JSONParseErrorReason::ExpectedBracket,
                })
            }
        }
        State::InObject => Err(JSONParseError {
            at: on.len(),
            reason: JSONParseErrorReason::ExpectedBracket,
        }),
        State::NumberValue { start } => {
            // TODO actual number handing
            let _result = cb(&key_chain, RootJSONValue::Number(&on[start..]));
            Ok(())
        }
        State::TrueFalseNull { start: _ } => Err(JSONParseError {
            at: on.len(),
            reason: JSONParseErrorReason::ExpectedTrueFalseNull,
        }),
    }
}
