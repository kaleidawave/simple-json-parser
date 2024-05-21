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
    ExpectedBracket,
    ExpectedTrueFalseNull,
    ExpectedValue,
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

/// # Errors
/// Returns an error if it tries to parse invalid JSON input
#[allow(clippy::too_many_lines)]
pub fn parse_with_exit_signal<'a>(
    on: &'a str,
    mut cb: impl for<'b> FnMut(&'b [JSONKey<'a>], RootJSONValue<'a>) -> bool,
) -> Result<(), JSONParseError> {
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
        None,
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

    let chars = on.char_indices();

    let mut key_chain = Vec::new();
    let mut state = State::None;

    for (idx, char) in chars {
        fn end_of_value(
            idx: usize,
            char: char,
            state: &mut State,
            key_chain: &mut Vec<JSONKey<'_>>,
        ) -> Result<(), JSONParseError> {
            if char == ',' {
                if let Some(JSONKey::Index(i)) = key_chain.last_mut() {
                    *i += 1;
                    *state = State::ExpectingValue;
                } else {
                    key_chain.pop();
                    *state = State::InObject;
                }
            } else if let ('}', Some(JSONKey::Slice(..))) = (char, key_chain.last()) {
                key_chain.pop();
            } else if let (']', Some(JSONKey::Index(..))) = (char, key_chain.last()) {
                key_chain.pop();
            } else if let c @ ('/' | '#') = char {
                key_chain.pop();
                *state = State::Comment {
                    last_was_asterisk: false,
                    start: idx,
                    multiline: false,
                    hash: c == '#',
                };
            } else if !char.is_whitespace() {
                return Err(JSONParseError {
                    at: idx,
                    reason: JSONParseErrorReason::ExpectedEndOfValue,
                });
            }
            Ok(())
        }

        match state {
            State::InKey {
                start,
                ref mut escaped,
            } => {
                if !*escaped && char == '"' {
                    key_chain.push(JSONKey::Slice(&on[start..idx]));
                    state = State::Colon;
                } else {
                    *escaped = char == '\\';
                }
            }
            State::StringValue {
                start,
                ref mut escaped,
            } => {
                if !*escaped && char == '"' {
                    state = State::EndOfValue;
                    let res = cb(&key_chain, RootJSONValue::String(&on[start..idx]));
                    if res {
                        return Ok(());
                    }
                } else {
                    *escaped = char == '\\';
                }
            }
            State::Colon => {
                if char == ':' {
                    state = State::ExpectingValue;
                } else if !char.is_whitespace() {
                    return Err(JSONParseError {
                        at: idx,
                        reason: JSONParseErrorReason::ExpectedColon,
                    });
                }
            }
            State::EndOfValue => {
                end_of_value(idx, char, &mut state, &mut key_chain)?;
            }
            State::Comment {
                ref mut last_was_asterisk,
                ref mut multiline,
                hash,
                start,
            } => {
                if char == '\n' && !*multiline {
                    if let Some(JSONKey::Index(..)) = key_chain.last() {
                        state = State::ExpectingValue;
                    } else {
                        state = State::InObject;
                    }
                } else if char == '*' && start + 1 == idx && !hash {
                    *multiline = true;
                } else if *multiline {
                    if *last_was_asterisk && char == '/' {
                        if let Some(JSONKey::Index(..)) = key_chain.last() {
                            state = State::ExpectingValue;
                        } else {
                            state = State::InObject;
                        }
                    } else {
                        *last_was_asterisk = char == '*';
                    }
                }
            }
            State::ExpectingValue => {
                state = match char {
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
                    char if char.is_whitespace() => state,
                    _ => {
                        return Err(JSONParseError {
                            at: idx,
                            reason: JSONParseErrorReason::ExpectedValue,
                        })
                    }
                }
            }
            State::InObject => {
                if char == '"' {
                    state = State::InKey {
                        escaped: false,
                        start: idx + '"'.len_utf8(),
                    };
                } else if char == '}' {
                    if let Some(JSONKey::Index(..)) = key_chain.last() {
                        state = State::ExpectingValue;
                    } else {
                        state = State::InObject;
                    }
                }
            }
            State::None => {
                if char == '{' {
                    state = State::InObject;
                } else if !char.is_whitespace() {
                    return Err(JSONParseError {
                        at: idx,
                        reason: JSONParseErrorReason::ExpectedBracket,
                    });
                }
            }
            State::NumberValue { start } => {
                // TODO actual number handing
                if char.is_whitespace() || matches!(char, '}' | ',' | ']') {
                    let res = cb(&key_chain, RootJSONValue::Number(&on[start..idx]));
                    if res {
                        return Ok(());
                    }
                    state = State::EndOfValue;
                    end_of_value(idx, char, &mut state, &mut key_chain)?;
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

    Ok(())
}
