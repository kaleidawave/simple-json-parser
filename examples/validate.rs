use simple_json_parser::{parse, JSONKey, JSONParseError};
use std::collections::HashSet;

fn main() {
    let sources: &[&str] = &[
        r#"{ "a": 2, "b": { "c": 5 }, "a": 4, "c": 5 }"#,
        // These almost yield the same chain **APART** from the string pointers
        r#"{ "a": { "b": 2, "c": 6 } }"#,
        r#"{ "a": { "b": 2 }, "a": { "c": 6 } }"#,
    ];
    for source in sources {
        eprintln!("{source} {duplicates:?}\n", duplicates = validate(source));
    }
}

// TODO compare HashSet vs Vec
// TODO can return more
fn validate<'a>(on: &'a str) -> Result<Vec<&'a str>, JSONParseError> {
    let mut seen_keys: Vec<HashSet<&'a str>> = Vec::new();
    let mut duplicate = Vec::new();

    let _result = parse(on, |keys, _| {
        if seen_keys.len() < keys.len() {
            seen_keys.extend((seen_keys.len()..keys.len()).map(|_| HashSet::new()));
        } else {
            seen_keys.truncate(keys.len());
        }
        for (seen, key) in seen_keys.iter_mut().zip(keys) {
            if let JSONKey::Slice(key) = key {
                if let Some(seen) = seen.replace(key) {
                    if seen.as_ptr() != key.as_ptr() {
                        duplicate.push(*key);
                    }
                }
            }
        }
    })?;
    Ok(duplicate)
}
