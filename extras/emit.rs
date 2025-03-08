use crate::{parse_advanced, JSONKey, ParseOptions, RootJSONValue};
use std::fmt::Write;

pub static EMPTY_NODE_PLACEHOLDER: &str = "simple-json-parse-empty-item";

#[cfg_attr(target_family = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub fn parse_and_emit(content: &str) -> String {
    let mut last: Vec<JSONKey<'_>> = Vec::new();
    // Fixes things as last only includes head
    let mut last_was_index = false;

    let mut buf = String::new();
    let indent = "\t";

    let pretty = true;

    // --- TEMP ---
    let options = ParseOptions {
        exit_on_first_value: true,
        allow_trailing_commas: true,
        partial_syntax: true,
        allow_comments: true,
        yield_comments: true,
        ..ParseOptions::default()
    };
    // --- TEMP ---

    let _ = parse_advanced(content, &options, |keys, value| {
        // TODO
        let current_key = keys.last();
        let head = &keys[..keys.len().saturating_sub(1)];
        if head == last {
            if last.len() == 0 && buf.is_empty() {
                write!(&mut buf, "{{").unwrap();
                if pretty {
                    write!(&mut buf, "\n{indent}").unwrap();
                }
            } else {
                if pretty {
                    write!(&mut buf, ",\n").unwrap();
                    for _ in 0..keys.len() {
                        write!(&mut buf, "{indent}").unwrap();
                    }
                } else {
                    write!(&mut buf, ",").unwrap();
                }
            }
            match current_key {
                Some(JSONKey::Slice(key)) => {
                    write!(&mut buf, "\"{key}\":").unwrap();
                    if pretty {
                        write!(&mut buf, " ").unwrap();
                    }
                }
                Some(JSONKey::Index(_)) => {}
                // For top level stuff
                None => {}
            }
        } else {
            let common = keys.iter().zip(last.iter()).filter(|(k, l)| k == l).count();
            let leaving = last.strip_prefix(&keys[..common]).unwrap();
            let entering = keys.strip_prefix(&keys[..common]).unwrap_or_default();

            {
                if !leaving.is_empty() {
                    let leaving = if last_was_index {
                        if pretty {
                            write!(&mut buf, "\n").unwrap();
                            for _ in 0..last.len() {
                                write!(&mut buf, "{indent}").unwrap();
                            }
                        }
                        write!(&mut buf, "]").unwrap();
                        &leaving[..leaving.len().saturating_sub(1)]
                    } else {
                        leaving
                    };
                    for (level, key) in leaving.iter().enumerate().rev() {
                        if pretty {
                            write!(&mut buf, "\n").unwrap();
                            for _ in 0..=(common + level) {
                                write!(&mut buf, "{indent}").unwrap();
                            }
                        }
                        write!(&mut buf, "{}", key_delimeters(key).close).unwrap();
                    }
                    last.truncate(common);
                }
            }

            write!(&mut buf, ",").unwrap();

            {
                for (idx, key) in entering.iter().enumerate() {
                    match key {
                        JSONKey::Slice(key) => {
                            if idx > 0 {
                                write!(&mut buf, "{{").unwrap();
                            }
                            if pretty {
                                write!(&mut buf, "\n").unwrap();
                                for _ in 0..=(common + idx) {
                                    write!(&mut buf, "{indent}").unwrap();
                                }
                            }
                            write!(&mut buf, "\"{key}\":").unwrap();
                            if pretty {
                                write!(&mut buf, " ").unwrap();
                            }
                        }
                        JSONKey::Index(_) => {
                            write!(&mut buf, "[").unwrap();
                            if pretty {
                                write!(&mut buf, "\n").unwrap();
                                for _ in 0..=(common + idx) {
                                    write!(&mut buf, "{indent}").unwrap();
                                }
                            }
                        }
                    }
                }
                last.extend_from_slice(&entering[..entering.len().saturating_sub(1)]);
            }
        }

        match value {
            RootJSONValue::String(s) => write!(&mut buf, "\"{s}\"").unwrap(),
            RootJSONValue::Number(n) => write!(&mut buf, "{n}").unwrap(),
            RootJSONValue::Boolean(b) => write!(&mut buf, "{b}").unwrap(),
            RootJSONValue::Null => write!(&mut buf, "null").unwrap(),
            RootJSONValue::Comment(comment) => write!(&mut buf, "// {comment}").unwrap(),
            RootJSONValue::Empty => write!(&mut buf, "\"{EMPTY_NODE_PLACEHOLDER}\"").unwrap(),
        }

        last_was_index = matches!(current_key, Some(JSONKey::Index(_)));
        false
    });

    if pretty {
        write!(&mut buf, "\n").unwrap();
        for _ in 0..last.len() {
            write!(&mut buf, "{indent}").unwrap();
        }
    }
    write!(
        &mut buf,
        "{close}",
        close = if last_was_index { ']' } else { '}' }
    )
    .unwrap();

    for (idx, key) in last.iter().enumerate().rev() {
        if pretty {
            write!(&mut buf, "\n").unwrap();
            for _ in 0..idx {
                write!(&mut buf, "{indent}").unwrap();
            }
        }
        write!(&mut buf, "{}", key_delimeters(key).close).unwrap();
    }

    buf
}

struct Delimeters {
    pub open: char,
    pub close: char,
}

#[rustfmt::skip]
fn key_delimeters(key: &JSONKey<'_>) -> Delimeters {
    match key {
        JSONKey::Slice(_) => Delimeters { open: '{', close: '}', },
        JSONKey::Index(_) => Delimeters { open: '[', close: ']', },
    }
}
