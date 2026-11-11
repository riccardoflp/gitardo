use super::*;
use crate::parse::{parse, parse_with_comments};
use crate::value::ValueKind;

/// A path written with dots between its keys, a key of digits only being an index: `list.0.name`.
fn path(written: &str) -> Vec<Key> {
    if written.is_empty() {
        return Vec::new();
    }
    let key = |part: &str| match part.parse::<usize>() {
        Ok(index) => Key::Index(index),
        Err(_) => Key::Member(part.to_string()),
    };
    written.split('.').map(key).collect()
}

fn set(text: &str, written_path: &str, value: &str) -> Result<String, Error> {
    set_value(text, &path(written_path), &parse(value).unwrap())
}

#[test]
fn setting_a_value_keeps_every_other_byte() {
    let cases = [
        (
            "// Settings\n{\n    // The size\n    \"size\": 12, // points\n\n    \"theme\": \"dark\"\n}\n",
            "size",
            "14",
            "// Settings\n{\n    // The size\n    \"size\": 14, // points\n\n    \"theme\": \"dark\"\n}\n",
        ),
        (
            "{\n  \"a\": 1,\n  \"b\": true\n}",
            "b",
            r#"{"x": [1]}"#,
            "{\n  \"a\": 1,\n  \"b\": {\n    \"x\": [\n      1\n    ]\n  }\n}",
        ),
        (
            "{\n    \"a\" /* k */ : /* v */ 1\n}",
            "a",
            "2",
            "{\n    \"a\" /* k */ : /* v */ 2\n}",
        ),
        (r#"{"a": 1, "a": 2}"#, "a", "3", r#"{"a": 1, "a": 3}"#),
        (
            r#"{"list": [1, 2, 3]}"#,
            "list.1",
            r#""two""#,
            r#"{"list": [1, "two", 3]}"#,
        ),
        (
            "{\n    \"ключ\": \"x\"\n}",
            "ключ",
            r#""値""#,
            "{\n    \"ключ\": \"値\"\n}",
        ),
        ("// c\n{\"a\": 1}\n", "", "[1]", "// c\n[1]\n"),
        (
            "{\n    \"a\": null\n}",
            "a",
            r#""line\nbreak""#,
            "{\n    \"a\": \"line\\nbreak\"\n}",
        ),
    ];
    for (text, written_path, value, expected) in cases {
        assert_eq!(
            set(text, written_path, value).unwrap(),
            expected,
            "set {written_path} in {text:?}"
        );
    }
}

#[test]
fn a_missing_member_is_added_at_the_end_of_its_object() {
    let cases = [
        ("{\n    \"a\": 1\n}", "{\n    \"a\": 1,\n    \"b\": 2\n}"),
        ("{\n    \"a\": 1,\n}", "{\n    \"a\": 1,\n    \"b\": 2,\n}"),
        ("{\n    \"a\": 1 // one\n}", "{\n    \"a\": 1, // one\n    \"b\": 2\n}"),
        (
            "{\n    \"a\": 1, // one\n}",
            "{\n    \"a\": 1, // one\n    \"b\": 2,\n}",
        ),
        (
            "{\n    \"a\": 1 /* one */\n}",
            "{\n    \"a\": 1, /* one */\n    \"b\": 2\n}",
        ),
        (
            "{\n    \"a\": 1\n    // end\n}",
            "{\n    \"a\": 1,\n    \"b\": 2\n    // end\n}",
        ),
        ("{\n    \"a\": 1}", "{\n    \"a\": 1,\n    \"b\": 2}"),
        ("{ \"a\": 1,\n  \"c\": 3\n}", "{ \"a\": 1,\n  \"c\": 3,\n  \"b\": 2\n}"),
        ("{\n\t\"a\": 1\n}", "{\n\t\"a\": 1,\n\t\"b\": 2\n}"),
        ("{\"a\": 1}", "{\"a\": 1, \"b\": 2}"),
        ("{\"a\": 1,}", "{\"a\": 1, \"b\": 2,}"),
        (
            "{\r\n    \"a\": 1\r\n}\r\n",
            "{\r\n    \"a\": 1,\r\n    \"b\": 2\r\n}\r\n",
        ),
        ("{}", "{\n    \"b\": 2\n}"),
        ("{ }", "{\n    \"b\": 2\n}"),
        ("{\n}", "{\n    \"b\": 2\n}"),
        ("{\r\n}", "{\r\n    \"b\": 2\r\n}"),
        ("{\n    // nothing yet\n}", "{\n    // nothing yet\n    \"b\": 2\n}"),
        ("// head\n{} // tail\n", "// head\n{\n    \"b\": 2\n} // tail\n"),
        ("\u{feff}{}", "\u{feff}{\n    \"b\": 2\n}"),
    ];
    for (text, expected) in cases {
        assert_eq!(set(text, "b", "2").unwrap(), expected, "add b to {text:?}");
    }
}

#[test]
fn missing_objects_on_the_way_are_added_in_the_style_of_the_file() {
    let cases = [
        (
            "{\n\t\"a\": 1\n}",
            "b.c.d",
            "true",
            "{\n\t\"a\": 1,\n\t\"b\": {\n\t\t\"c\": {\n\t\t\t\"d\": true\n\t\t}\n\t}\n}",
        ),
        (
            "{\n  \"editor\": {\n    \"size\": 1\n  }\n}",
            "editor.font",
            "\"mono\"",
            "{\n  \"editor\": {\n    \"size\": 1,\n    \"font\": \"mono\"\n  }\n}",
        ),
        (
            "{\r\n    \"a\": 1\r\n}\r\n",
            "c",
            r#"{"d": 1}"#,
            "{\r\n    \"a\": 1,\r\n    \"c\": {\r\n        \"d\": 1\r\n    }\r\n}\r\n",
        ),
        (
            "{\"a\": 1}",
            "b",
            r#"{"c": [1, 2]}"#,
            "{\"a\": 1, \"b\": {\"c\": [1, 2]}}",
        ),
        ("{\"a\": {}}", "a.b", "1", "{\"a\": {\"b\": 1}}"),
        (
            "{\n    \"a\": {}\n}",
            "a.b",
            "1",
            "{\n    \"a\": {\n        \"b\": 1\n    }\n}",
        ),
        (
            "{\n    \"ключ\": 1\n}",
            "😀",
            "2",
            "{\n    \"ключ\": 1,\n    \"😀\": 2\n}",
        ),
    ];
    for (text, written_path, value, expected) in cases {
        assert_eq!(
            set(text, written_path, value).unwrap(),
            expected,
            "set {written_path} in {text:?}"
        );
    }
}

#[test]
fn items_are_added_at_the_end_of_their_array() {
    let cases = [
        (
            "{\n    \"list\": [\n        1\n    ]\n}",
            "list.1",
            "2",
            "{\n    \"list\": [\n        1,\n        2\n    ]\n}",
        ),
        (
            "{\n    \"list\": []\n}",
            "list.0",
            "2",
            "{\n    \"list\": [\n        2\n    ]\n}",
        ),
        (
            "{}",
            "list.0.name",
            "\"x\"",
            "{\n    \"list\": [\n        {\n            \"name\": \"x\"\n        }\n    ]\n}",
        ),
    ];
    for (text, written_path, value, expected) in cases {
        assert_eq!(
            set(text, written_path, value).unwrap(),
            expected,
            "set {written_path} in {text:?}"
        );
    }
}

#[test]
fn a_text_with_no_value_becomes_an_object() {
    let cases = [
        ("", "a", "1", "{\n    \"a\": 1\n}\n"),
        ("  \n\t", "a", "1", "{\n    \"a\": 1\n}\n"),
        (
            "\r\n",
            "a.b",
            "1",
            "{\r\n    \"a\": {\r\n        \"b\": 1\r\n    }\r\n}\r\n",
        ),
        ("", "", "[]", "[]\n"),
        ("// settings\n", "a", "1", "// settings\n{\n    \"a\": 1\n}\n"),
        ("/* settings */", "a", "1", "/* settings */\n{\n    \"a\": 1\n}\n"),
        ("\u{feff}", "a", "1", "\u{feff}{\n    \"a\": 1\n}\n"),
        ("\u{feff}// c\n", "a", "1", "\u{feff}// c\n{\n    \"a\": 1\n}\n"),
    ];
    for (text, written_path, value, expected) in cases {
        assert_eq!(
            set(text, written_path, value).unwrap(),
            expected,
            "set {written_path} in {text:?}"
        );
    }
}

/// The error for a path that goes through something other than an object or an array.
fn not_a_container(written_path: &str, found: ValueKind, key: &str) -> Error {
    let key = path(key).remove(0);
    Error::NotAContainer {
        path: path(written_path),
        found,
        key,
    }
}

#[test]
fn a_path_through_something_else_is_refused() {
    let cases = [
        (r#"{"a": "text"}"#, "a.b", not_a_container("a", ValueKind::String, "b")),
        (r#"{"a": [1]}"#, "a.x", not_a_container("a", ValueKind::Array, "x")),
        (r#"{"a": {}}"#, "a.0", not_a_container("a", ValueKind::Object, "0")),
        ("1", "a", not_a_container("", ValueKind::Number, "a")),
        (r#"{"a": null}"#, "a.b.c", not_a_container("a", ValueKind::Null, "b")),
        ("[true]", "0.b", not_a_container("0", ValueKind::Bool, "b")),
    ];
    for (text, written_path, error) in cases {
        assert_eq!(
            set(text, written_path, "1"),
            Err(error.clone()),
            "set {written_path} in {text:?}"
        );
        assert_eq!(
            remove_value(text, &path(written_path)),
            Err(error),
            "remove {written_path} in {text:?}"
        );
    }
}

#[test]
fn an_index_more_than_one_past_the_end_is_refused() {
    let cases = [
        (
            "[1]",
            "3",
            Error::IndexOutOfRange {
                path: vec![],
                index: 3,
                length: 1,
            },
        ),
        (
            "{}",
            "a.2",
            Error::IndexOutOfRange {
                path: path("a"),
                index: 2,
                length: 0,
            },
        ),
        (
            r#"{"a": []}"#,
            "a.0.1",
            Error::IndexOutOfRange {
                path: path("a.0"),
                index: 1,
                length: 0,
            },
        ),
    ];
    for (text, written_path, error) in cases {
        assert_eq!(
            set(text, written_path, "1"),
            Err(error),
            "set {written_path} in {text:?}"
        );
    }
}

#[test]
fn a_text_that_is_not_json_with_comments_is_refused() {
    for text in ["{\"a\" 1}", "{", "{} {}", "/* open"] {
        assert!(
            matches!(set(text, "a", "1"), Err(Error::Syntax { .. })),
            "set in {text:?}"
        );
        assert!(
            matches!(remove_value(text, &path("a")), Err(Error::Syntax { .. })),
            "remove in {text:?}"
        );
    }
}

#[test]
fn removing_a_member_removes_its_line_and_fixes_the_commas() {
    let three = "{\n    \"a\": 1,\n    \"b\": 2,\n    \"c\": 3\n}";
    let cases = [
        (three, "b", "{\n    \"a\": 1,\n    \"c\": 3\n}"),
        (three, "c", "{\n    \"a\": 1,\n    \"b\": 2\n}"),
        (three, "a", "{\n    \"b\": 2,\n    \"c\": 3\n}"),
        (
            "{\n    \"a\": 1, // about a\n    \"b\": 2\n}",
            "b",
            "{\n    \"a\": 1 // about a\n}",
        ),
        ("{\n    \"a\": 1, // one\n    \"b\": 2\n}", "a", "{\n    \"b\": 2\n}"),
        (
            "{\n    \"a\": 1,\n    // about b\n    \"b\": 2\n}",
            "b",
            "{\n    \"a\": 1\n    // about b\n}",
        ),
        ("{\n    \"a\": 1,\n    \"b\": 2,\n}", "b", "{\n    \"a\": 1,\n}"),
        (
            "{\r\n    \"a\": 1,\r\n    \"b\": 2\r\n}\r\n",
            "b",
            "{\r\n    \"a\": 1\r\n}\r\n",
        ),
        ("{\n    \"a\": 1,\n    \"b\": 2 }", "b", "{\n    \"a\": 1 }"),
        (
            "{\n    \"x\": {\n        \"y\": 1,\n        \"z\": 2\n    }\n}",
            "x.y",
            "{\n    \"x\": {\n        \"z\": 2\n    }\n}",
        ),
        ("{\n    \"ключ\": 1,\n    \"😀\": 2\n}", "😀", "{\n    \"ключ\": 1\n}"),
    ];
    for (text, written_path, expected) in cases {
        assert_eq!(
            remove_value(text, &path(written_path)).unwrap(),
            expected,
            "remove {written_path} from {text:?}"
        );
    }
}

#[test]
fn removing_from_one_line_keeps_it_on_one_line() {
    let cases = [
        (r#"{"a": 1, "b": 2, "c": 3}"#, "a", r#"{"b": 2, "c": 3}"#),
        (r#"{"a": 1, "b": 2, "c": 3}"#, "b", r#"{"a": 1, "c": 3}"#),
        (r#"{"a": 1, "b": 2, "c": 3}"#, "c", r#"{"a": 1, "b": 2}"#),
        (r#"{"a": 1, "b": 2,}"#, "b", r#"{"a": 1,}"#),
        ("[1, 2, 3]", "1", "[1, 3]"),
        ("[1, 2, 3]", "2", "[1, 2]"),
    ];
    for (text, written_path, expected) in cases {
        assert_eq!(
            remove_value(text, &path(written_path)).unwrap(),
            expected,
            "remove {written_path} from {text:?}"
        );
    }
}

#[test]
fn removing_the_only_member_closes_the_object_up() {
    let cases = [
        ("{\n    \"a\": 1\n}", "{}"),
        ("{\n    \"a\": 1 // about a\n}\n", "{}\n"),
        ("{\"a\": 1}", "{}"),
        ("{ \"a\": 1, }", "{}"),
        ("{\n    // keep\n    \"a\": 1\n}", "{\n    // keep\n}"),
        ("{\n    \"a\": 1\n    /* keep */\n}", "{\n    /* keep */\n}"),
    ];
    for (text, expected) in cases {
        assert_eq!(
            remove_value(text, &path("a")).unwrap(),
            expected,
            "remove a from {text:?}"
        );
    }
}

#[test]
fn removing_what_is_not_there_changes_nothing() {
    let cases = [
        ("{\"a\": 1}", "b"),
        ("[1]", "5"),
        ("", "a"),
        ("// only\n", "a"),
        ("{\"a\": {}}", "a.b.c"),
    ];
    for (text, written_path) in cases {
        assert_eq!(
            remove_value(text, &path(written_path)).unwrap(),
            text,
            "remove {written_path} from {text:?}"
        );
    }
    assert_eq!(remove_value("{}", &[]), Err(Error::EmptyPath));
}

#[test]
fn edits_read_back_as_the_values_they_set() {
    let text = "// Settings\n{\n    \"graph\": { \"pageSize\": 500 }, // rows\n    \"list\": [1,],\n}\n";
    let edited = set(text, "graph.pageSize", "1000").unwrap();
    let edited = set(&edited, "graph.combine", "false").unwrap();
    let edited = set(&edited, "list.1", "2").unwrap();
    let edited = remove_value(&edited, &path("list.0")).unwrap();
    let value = parse_with_comments(&edited).unwrap();
    assert_eq!(
        value.get("graph").and_then(|graph| graph.get("pageSize")),
        Some(&Value::from(1000_u64))
    );
    assert_eq!(
        value.get("graph").and_then(|graph| graph.get("combine")),
        Some(&Value::from(false))
    );
    assert_eq!(value.get("list"), Some(&Value::Array(vec![Value::from(2_u64)])));
    assert!(edited.starts_with("// Settings\n{\n"), "{edited}");
    assert!(edited.contains("}, // rows\n"), "{edited}");
}
