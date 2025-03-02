use simple_json_parser::{parse_advanced, JSONKey, ParseOptions, RootJSONValue};

#[test]
fn at_end_of_value() {
    // For example
    let source =
        r#"<script id="data" type="application/json">{"org": 10, "items":["one"]}</script>"#;
    let source = source
        .strip_prefix(r#"<script id="data" type="application/json">"#)
        .unwrap();

    let mut values = 2;
    let options = ParseOptions {
        exit_on_first_value: true,
        ..Default::default()
    };
    let (result, _none) = parse_advanced::<()>(source, &options, |keys, value| {
        values -= 1;
        if let &[JSONKey::Slice("org")] = keys {
            assert_eq!(value, RootJSONValue::Number("10"));
        } else if let &[JSONKey::Slice("items"), JSONKey::Index(0)] = keys {
            assert_eq!(value, RootJSONValue::String("one"));
        } else {
            panic!("Unknown value {keys:?}");
        }
        None
    })
    .unwrap();

    assert_eq!(values, 0);
    assert_eq!(&source[result..], "</script>");
}

#[test]
fn at_found_value() {
    // For example
    let source = r#"{"org": 10, "items":[4, { "name": "Ben" }, 6]}"#;

    let mut values = 2;
    let (parsed, result) =
        parse_advanced::<RootJSONValue<'_>>(source, &ParseOptions::default(), |keys, value| {
            if let &[JSONKey::Slice("items"), JSONKey::Index(1), JSONKey::Slice("name")] = keys {
                Some(value)
            } else {
                values -= 1;
                None
            }
        })
        .unwrap();

    assert_eq!(
        result.expect("No found value"),
        RootJSONValue::String("Ben")
    );
    assert_eq!(values, 0);
    assert_eq!(&source[parsed..], " }, 6]}");
}
