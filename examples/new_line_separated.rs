use simple_json_parser::{parse_with_options, ParseOptions};

fn main() {
    let to_parse = r#"{ "x": 2 }
{ "y": 3, "y2": 5 }
{ "z": 4 }"#;

    let options = ParseOptions {
        top_level_separator: Some("\n"),
        ..Default::default()
    };

    let result = parse_with_options::<()>(to_parse, &options, |keys, value| {
        eprintln!("{keys:?} -> {value:?}");
        None
    });

    eprintln!("{result:?}");
}
