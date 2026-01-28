#![doc = include_str!("./README.md")]

use std::borrow::Cow;

/// A identifier into JSON
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum JSONKey<'a> {
    /// From `"x": ..`
    Slice(&'a str),
    /// From `[..]`
    Index(usize),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct JSONString<'a>(&'a str);

impl<'a> JSONString<'a> {
    #[must_use]
    pub fn raw(self) -> &'a str {
        self.0
    }

    /// The unescaped value
    #[must_use]
    pub fn value(self) -> Cow<'a, str> {
        unescape_string_content(self.0)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct JSONNumber<'a>(&'a str);

impl<'a> JSONNumber<'a> {
    #[must_use]
    pub fn raw(self) -> &'a str {
        self.0
    }

    /// This can fail. Initial parsing does not check numbers are well formed
    /// for utility and performance reasons
    ///
    /// # Errors
    ///
    /// returns an `Err` if `str::parse::<f64>` returns `Err`
    pub fn value_checked(self) -> Result<f64, std::num::ParseFloatError> {
        str::parse(self.0)
    }

    /// Value assuming source is well formed
    ///
    /// # Panics
    ///
    /// panics if `str::parse::<f64>` returns `Err`
    #[must_use]
    pub fn value_unwrap(self) -> f64 {
        self.value_checked().unwrap()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum RootJSONValue<'a> {
    String(JSONString<'a>),
    Number(JSONNumber<'a>),
    Boolean(bool),
    Null,
    EmptyObject,
    EmptyArray,
    /// Under `yield_comments` these are *sometimes* emitted as to preserve
    /// information during formatting
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
    ExpectedKey,
    ExpectedValue,
    InvalidComment,
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
    /// returns "empty" variant on
    pub partial_syntax: bool,
    pub allow_comments: bool,
    // TODO combine with above
    pub yield_comments: bool,
    // For new line JSON etc
    pub top_level_separator: Option<char>,
    // pub top_level_separator: Option<&'static str>,
}

/// Returns the number of bytes parsed
///
/// # Errors
/// Returns an error if it tries to parse invalid JSON input
pub fn parse_with_options<'a, T>(
    on: &'a str,
    options: &ParseOptions,
    mut cb: impl for<'b> FnMut(&'b [JSONKey<'a>], RootJSONValue<'a>) -> Option<T>,
) -> Result<(usize, Option<T>), JSONParseError> {
    /// Does not check string is empty etc
    fn find_non_escaped_quoted(on: &str) -> Option<usize> {
        let mut characters = on.bytes().enumerate();
        while let Some((idx, chr)) = characters.next() {
            if b'\\' == chr {
                let _ = characters.next();
            } else if b'"' == chr {
                return Some(idx);
            }
        }
        None
    }

    fn parse_comment(on: &str) -> Option<(&str, usize)> {
        if on.starts_with('#') {
            let offset = on.find('\n').unwrap_or(on.len());
            Some((&on[..offset][1..], offset))
        } else if on.starts_with("//") {
            let offset = on.find('\n').unwrap_or(on.len());
            Some((&on[..offset][2..], offset))
        } else if let Some(rest) = on.strip_prefix("/*") {
            let offset = rest.find("*/")?;
            Some((&rest[..offset], offset + 4))
        } else {
            None
        }
    }

    let (mut idx, mut key_chain, mut in_object): (usize, Vec<JSONKey<'_>>, bool) =
        Default::default();
    let (bytes, tls) = (on.as_bytes(), options.top_level_separator);

    if tls.is_some() {
        key_chain.push(JSONKey::Index(0));
    }

    macro_rules! emit {
        ($item:expr) => {
            // TODO pass idx
            let res = cb(&key_chain, $item);
            if res.is_some() {
                return Ok((idx, res));
            }
        };
    }

    macro_rules! return_err {
        ($reason:ident) => {
            return Err(JSONParseError {
                at: idx,
                reason: JSONParseErrorReason::$reason,
            });
        };
    }

    macro_rules! skip_whitespace_find_comments {
        () => {
            while let Some(byte) = bytes.get(idx) {
                if let b' ' | b'\t' | b'\r' | b'\n' = byte {
                    idx += 1;
                } else if let b'/' | b'#' = byte {
                    let Some((comment, offset)) = parse_comment(&on[idx..]) else {
                        return_err!(InvalidComment);
                    };
                    idx += offset;
                    if options.yield_comments {
                        emit!(RootJSONValue::Comment(comment));
                    }
                } else {
                    break;
                }
            }
        };
    }

    skip_whitespace_find_comments!();

    while idx < bytes.len() {
        if in_object {
            skip_whitespace_find_comments!();
            if let Some(b'"') = bytes.get(idx) {
                let rest = &on[1..][idx..];
                let Some(offset) = find_non_escaped_quoted(rest) else {
                    return_err!(ExpectedQuote);
                };
                key_chain.push(JSONKey::Slice(&rest[..offset]));
                idx += offset + 2;
                skip_whitespace_find_comments!();
                if on.as_bytes().get(idx).copied().unwrap_or_default() != b':' {
                    // TODO partial could find next ':'?
                    return_err!(ExpectedColon);
                }
                idx += 1;
            } else {
                return_err!(ExpectedKey);
            }
        }

        skip_whitespace_find_comments!();

        match bytes.get(idx).copied() {
            Some(b'{') => {
                idx += 1;
                // little hack
                skip_whitespace_find_comments!();
                if let Some(b'}') = bytes.get(idx) {
                    idx += 1;
                    emit!(RootJSONValue::EmptyObject);
                } else {
                    in_object = true;
                    continue;
                }
            }
            Some(b'[') => {
                idx += 1;
                key_chain.push(JSONKey::Index(0));
                in_object = false;
                continue;
            }
            Some(b']') => {
                idx += 1;
                match key_chain.pop() {
                    Some(JSONKey::Index(0)) => {
                        emit!(RootJSONValue::EmptyArray);
                    }
                    Some(JSONKey::Index(_)) if options.allow_trailing_commas => {}
                    _ => {
                        return_err!(ExpectedEndOfValue);
                    }
                }
                in_object = matches!(key_chain.last(), Some(JSONKey::Slice(_)));
            }
            Some(b'"') => {
                let rest = &on[idx..][1..];
                let Some(offset) = find_non_escaped_quoted(rest) else {
                    return_err!(ExpectedEndOfValue);
                };
                idx += offset + 2;
                emit!(RootJSONValue::String(JSONString(&rest[..offset])));
            }
            Some(b'0'..=b'9' | b'-') => {
                /// can include bad values
                fn find_non_number(chr: char) -> bool {
                    !matches!(chr, '0'..='9' | '.' | 'e' | 'E' | '+' | '-')
                }

                let rest = &on[idx..];
                let Some(offset) = rest.find(find_non_number) else {
                    return_err!(ExpectedEndOfValue);
                };
                idx += offset;
                emit!(RootJSONValue::Number(JSONNumber(&rest[..offset])));
            }
            Some(b't') if on[idx..].starts_with("true") => {
                idx += 4;
                emit!(RootJSONValue::Boolean(true));
            }
            Some(b'f') if on[idx..].starts_with("false") => {
                idx += 5;
                emit!(RootJSONValue::Boolean(false));
            }
            Some(b'n') if on[idx..].starts_with("null") => {
                idx += 4;
                emit!(RootJSONValue::Null);
            }
            Some(b @ (b',' | b'}')) if options.partial_syntax => {
                emit!(RootJSONValue::Empty);
                idx += 1;
                if in_object {
                    let _ = key_chain.pop();
                }
                if b == b',' {
                    continue;
                }
            }
            _ => {
                return_err!(ExpectedValue);
            }
        }

        while let Some(byte) = bytes.get(idx) {
            if key_chain.is_empty() {
                return Ok((idx, None));
            }

            if tls.is_some_and(|c: char| on[idx..].starts_with(c)) {
                idx += 1;
                if let [JSONKey::Index(ref mut idx)] = key_chain.as_mut_slice() {
                    *idx += 1;
                    break;
                }
            } else if let b' ' | b'\t' | b'\r' | b'\n' = byte {
                idx += 1;
            } else if let b'/' | b'#' = byte {
                let Some((comment, offset)) = parse_comment(&on[idx..]) else {
                    return_err!(InvalidComment);
                };
                idx += offset;
                if options.yield_comments {
                    emit!(RootJSONValue::Comment(comment));
                }
            } else {
                let new_byte = if *byte == b',' {
                    idx += 1;
                    if let Some(JSONKey::Index(ref mut idx)) = key_chain.last_mut() {
                        *idx += 1;
                    } else {
                        key_chain.pop();
                    }
                    if !options.allow_trailing_commas {
                        break;
                    }
                    skip_whitespace_find_comments!();
                    let Some(b @ (b'}' | b']')) = bytes.get(idx) else {
                        break;
                    };
                    *b
                } else {
                    *byte
                };
                match new_byte {
                    b'}' if *byte == b',' || in_object => {}
                    b']' if matches!(key_chain.last(), Some(JSONKey::Index(_))) => {}
                    _ => {
                        return_err!(ExpectedEndOfValue);
                    }
                }
                idx += 1;
                key_chain.pop();
                in_object = matches!(key_chain.last(), Some(JSONKey::Slice(_)));
            }
        }
    }

    let tl = tls.is_some_and(|_| matches!(key_chain.as_slice(), &[JSONKey::Index(_)]));

    if !(key_chain.is_empty() || tl) {
        return_err!(ExpectedBracket);
    }

    Ok((on.len(), None))
}

/// Equates key chains while accounting for escapes. TODO no unicode thingies
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
        if index < start {
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
            // 📸
            Some('q') => {
                result += "\"";
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
                    .get(0..4)
                    .and_then(|slice| parse_hex(slice).ok())
                    .and_then(char::from_u32);

                if let Some(item) = unicode_char {
                    result.to_mut().push(item);
                    start = index + 6;
                } else {
                    start = index;
                    eprintln!("expected 4 hex digits");
                }
            }
            Some(chr) => {
                start = index + 1;
                eprintln!("unexpected item {chr:?}");
            }
            // This is unreachable with the results returned from JSON parsing
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
        assert!(matches!(
            unescape_string_content("No quotes here"),
            Cow::Borrowed(_)
        ));
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
        assert_eq!(unescape_string_content("hex\\u0021"), "hex!");
        assert_eq!(unescape_string_content("\\t\\t"), "\t\t");
    }

    #[test]
    fn unescaping_unknown_or_invalid() {
        assert_eq!(unescape_string_content("not \\an escape"), "not an escape");
        assert_eq!(
            unescape_string_content("not \\u{34} escape"),
            "not \\u{34} escape"
        );
    }

    #[test]
    fn unescaping_end() {
        assert_eq!(unescape_string_content("ends with \\"), "ends with \\");
    }
}
