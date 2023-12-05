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
pub enum JSONParseError {
    ExpectedColon,
    ExpectedEndOfValue,
    ExpectedBracket,
    ExpectedTrueFalseNull,
    ExpectedValue,
}

pub fn parse<'a>(
    on: &'a str,
    mut cb: impl for<'b> FnMut(&'b [JSONKey<'a>], RootJSONValue<'a>),
) -> Result<(), (usize, JSONParseError)> {
    let chars = on.char_indices();

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

    let mut key_chain = Vec::new();
    let mut state = State::None;

    for (idx, char) in chars {
        fn end_of_value(
            idx: usize,
            char: char,
            state: &mut State,
            key_chain: &mut Vec<JSONKey<'_>>,
        ) -> Result<(), (usize, JSONParseError)> {
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
            } else if !char.is_whitespace() {
                return Err((idx, JSONParseError::ExpectedEndOfValue));
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
                    cb(&key_chain, RootJSONValue::String(&on[start..idx]));
                } else {
                    *escaped = char == '\\';
                }
            }
            State::Colon => {
                if char == ':' {
                    state = State::ExpectingValue;
                } else if !char.is_whitespace() {
                    return Err((idx, JSONParseError::ExpectedColon));
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
                        state = State::ExpectingValue
                    } else {
                        state = State::InObject
                    }
                } else if char == '*' && start + 1 == idx && !hash {
                    *multiline = true
                } else if *multiline {
                    if *last_was_asterisk && char == '/' {
                        if let Some(JSONKey::Index(..)) = key_chain.last() {
                            state = State::ExpectingValue
                        } else {
                            state = State::InObject
                        }
                    } else {
                        *last_was_asterisk = char == '*'
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
                    char => {
                        if !char.is_whitespace() {
                            return Err((idx, JSONParseError::ExpectedValue));
                        } else {
                            state
                        }
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
                        state = State::ExpectingValue
                    } else {
                        state = State::InObject
                    }
                }
            }
            State::None => {
                if char == '{' {
                    state = State::InObject;
                } else if !char.is_whitespace() {
                    return Err((idx, JSONParseError::ExpectedBracket));
                }
            }
            State::NumberValue { start } => {
                // TODO actual number handing
                if matches!(char, '\n' | ' ' | '}' | ',' | ']') {
                    cb(&key_chain, RootJSONValue::Number(&on[start..idx]));
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
                            cb(&key_chain, RootJSONValue::True);
                            state = State::EndOfValue;
                        }
                        "null" => {
                            cb(&key_chain, RootJSONValue::Null);
                            state = State::EndOfValue;
                        }
                        "fals" => {}
                        _ => return Err((idx, JSONParseError::ExpectedTrueFalseNull)),
                    }
                } else if let "false" = &on[start..=idx] {
                    cb(&key_chain, RootJSONValue::False);
                    state = State::EndOfValue;
                } else {
                    return Err((idx, JSONParseError::ExpectedTrueFalseNull));
                }
            }
        }
    }

    Ok(())
}
