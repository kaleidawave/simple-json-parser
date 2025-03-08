//! Additional JSON utilies
pub mod emit;

/// temp testing
#[cfg_attr(target_family = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub fn count_values(content: &str) -> usize {
    let partial_syntax = true;
    let allow_comments = true;

    let options = crate::ParseOptions {
        partial_syntax,
        allow_comments,
        yield_comments: allow_comments,
        ..Default::default()
    };

    let mut count = 0;
    let _result = crate::parse_advanced(content, &options, |_, _| {
        count += 1;
        false
    });

    count
}
