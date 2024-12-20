use simple_json_parser::parse;

fn main() {
    let to_parse = &[
        "199",
        "\"Hiya\"",
        "[1, 2, \"something\"]",
        "true",
        "false",
        "null",
    ];

    for item in to_parse {
        eprintln!("parsing {item} as JSON");
        let result = parse(item, |keys, value| {
            eprintln!("{keys:?} -> {value:?}");
        });

        assert!(result.is_ok());
    }
}
