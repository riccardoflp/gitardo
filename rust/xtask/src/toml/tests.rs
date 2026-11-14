use super::{Value, parse};

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

#[test]
fn reads_tables_and_their_keys() {
    let entries = parse("name = \"app\"\n\n[package]\nedition = \"2024\" # a comment\n").unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].table, Vec::<String>::new());
    assert_eq!(entries[0].group, 0);
    assert_eq!(entries[1].table, vec!["package"]);
    assert_eq!(entries[1].key, vec!["edition"]);
    assert_eq!(entries[1].value, text("2024"));
    assert_eq!(entries[1].line, 4);
}

#[test]
fn tells_the_elements_of_an_array_of_tables_apart() {
    let lock = "version = 4\n\n[[package]]\nname = \"a\"\n\n[[package]]\nname = \"b\"\nsource = \"registry+x\"\n";
    let entries = parse(lock).unwrap();
    let groups: Vec<usize> = entries.iter().map(|entry| entry.group).collect();
    assert_eq!(groups, vec![0, 1, 2, 2]);
    assert_eq!(entries[0].value, Value::Bare(String::from("4")));
}

#[test]
fn reads_arrays_over_several_lines_with_comments() {
    let entries = parse("dependencies = [\n \"a\", # first\n \"b 1.0\",\n]\n").unwrap();
    assert_eq!(entries[0].value, Value::Array(vec![text("a"), text("b 1.0")]));
}

#[test]
fn reads_inline_tables_and_dotted_keys() {
    let entries =
        parse("[dependencies]\njson = { path = \"../json\", optional = true }\nring.workspace = true\n").unwrap();
    assert_eq!(entries[0].value.member("path"), Some(&text("../json")));
    assert_eq!(entries[0].value.member("optional"), Some(&Value::Boolean(true)));
    assert_eq!(entries[1].key, vec!["ring", "workspace"]);
}

#[test]
fn reads_quoted_segments_of_table_names() {
    let entries = parse("[target.'cfg(windows)'.dependencies]\nx = \"1\"\n[a.\"b.c\"]\ny = 'z'\n").unwrap();
    assert_eq!(entries[0].table, vec!["target", "cfg(windows)", "dependencies"]);
    assert_eq!(entries[1].table, vec!["a", "b.c"]);
    assert_eq!(entries[1].value, text("z"));
}

#[test]
fn resolves_escapes_and_multi_line_strings() {
    let entries = parse("a = \"tab\\there \\u00e8\"\nb = \"\"\"\nfirst\nsecond\"\"\"\nc = '''raw\\n'''\n").unwrap();
    assert_eq!(entries[0].value, text("tab\there è"));
    assert_eq!(entries[1].value, text("first\nsecond"));
    assert_eq!(entries[2].value, text("raw\\n"));
    assert_eq!(entries[2].line, 5);
}

#[test]
fn says_on_which_line_the_text_is_malformed() {
    let cases = [
        ("a = \"open\n", 1),
        ("ok = 1\n\n[table\n", 3),
        ("a = \"x\" b\n", 1),
        ("= 1\n", 1),
        ("a = [\"x\" \"y\"]\n", 1),
    ];
    for (text, line) in cases {
        let malformed = parse(text).unwrap_err();
        assert_eq!(malformed.line, line, "{text:?}");
    }
}

#[test]
fn reads_crlf_files() {
    let entries = parse("[package]\r\nname = \"app\"\r\n").unwrap();
    assert_eq!(entries[0].value, text("app"));
    assert_eq!(entries[0].line, 2);
}
