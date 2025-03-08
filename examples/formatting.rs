//! TODO abstract with key_delimeters, indent function, performance test, comments (& others in book)

fn main() {
    let content = if let Some(path) = std::env::args().nth(1) {
        std::borrow::Cow::Owned(std::fs::read_to_string(path).unwrap())
    } else {
        std::borrow::Cow::Borrowed(
            r#"{ "a": 1, "b": { "c": 2, "e": 5, "g": { "x": 1000 }, "f": 7 }, "d": [3, 4] }"#,
        )
    };

    let pretty = std::env::args().skip(1).any(|flag| flag == "--pretty");

    // eprintln!("og  {content}");
    // eprintln!("new {buf}");
}
