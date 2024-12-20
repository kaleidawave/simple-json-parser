use simple_json_parser::parse;

fn main() {
    let content = r#"// Something
    {
        "a": 2,
        // Another
        "b": 5,
        # another comment
    }"#;

    let result = parse(content, |keys, value| {
        eprintln!("{keys:?} -> {value:?}");
    });

    assert!(result.is_ok());
}
