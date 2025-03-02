use simple_json_parser::{parse_advanced, JSONKey, ParseOptions, RootJSONValue};

#[test]
fn disable_comments() {
    let source = r#"{
        // some comment
        "hi": "Ben"
    }"#;

    let with_comments = parse_advanced::<()>(
        source,
        &ParseOptions {
            allow_comments: true,
            ..Default::default()
        },
        |keys, value| {
            if let &[JSONKey::Slice("hi")] = keys {
                assert_eq!(value, RootJSONValue::String("Ben"));
            } else {
                panic!()
            }
            None
        },
    );
    let without_comments = parse_advanced::<()>(
        source,
        &ParseOptions {
            allow_comments: false,
            ..Default::default()
        },
        |_keys, _value| {
            eprintln!("{:?}", (_keys, _value));
            None
        },
    );

    assert!(with_comments.is_ok());
    assert!(without_comments.is_err());
}
