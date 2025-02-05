use simple_json_parser::{parse_with_exit_signal, ParseOptions};

#[test]
fn parse_package_json() {
    let content = r#"{
    "a": 2,
    "b": [],
    "c": 5,
    "d": {},
    "e": "",
    "f": true,
    "g": 4,
    "h": "x \\",
    "i": 7
}"#;

    let mut found = 0;
    let result = parse_with_exit_signal(
        content,
        |_keys, _value| {
            found += 1;
            false
        },
        &ParseOptions::default(),
    );

    assert!(result.is_ok());
    assert_eq!(found, 7);
}
