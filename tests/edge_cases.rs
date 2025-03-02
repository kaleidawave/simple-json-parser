use simple_json_parser::{parse_advanced, ParseOptions};

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
    let result = parse_advanced::<()>(content, &ParseOptions::default(), |_keys, _value| {
        found += 1;
        None
    });

    assert!(result.is_ok());
    assert_eq!(found, 7);
}

use simple_json_parser::{parse, JSONParseError};

#[test]
fn good_cases() {
    let good_cases: &[&str] = &[
        r#"{ "a": -6 }"#,
        r#"{ "a": 6 }"#,
        r#"{ "a": 6e10 }"#,
        r#"{ "a": 10, "hello": 16 }"#,
        r#"{ "a": [] }"#,
        r#"{ "b": {} }"#,
    ];

    for case in good_cases {
        let result = parse(case, |keys, value| eprintln!("{keys:?} -> {value:?}"));

        match result {
            Ok(_) => {
                eprintln!("✅ Valid script did not error")
            }
            Err(JSONParseError { at, reason }) => {
                panic!("❌ Valid script error'ed {reason:?} @ {at}");
            }
        }
    }
}

#[test]
fn bad_cases() {
    let bad_cases: &[&str] = &[
        // Trailing commas (TODO should be enabled under feature)
        r#"{ "a": 6, }"#,
        r#"{ "a": 6NOT_A_NUMBER }"#,
        r#"{ } }"#,
        r#"{ ]"#,
    ];

    for case in bad_cases {
        let result = parse(case, |keys, value| eprintln!("{keys:?} -> {value:?}"));

        match result {
            Ok(_) => {
                eprintln!("❌ Invalid script did not error")
            }
            Err(JSONParseError { at, reason }) => {
                eprintln!("✅ Invalid script error'ed {reason:?} @ {at}");
            }
        }
    }
}
