use super::*;
use crate::value::Number;

fn syntax(line: usize, column: usize, problem: Problem) -> Error {
    Error::Syntax {
        position: Position { line, column },
        problem,
    }
}

fn object(members: &[(&str, Value)]) -> Value {
    Value::Object(
        members
            .iter()
            .map(|(name, value)| Member {
                name: (*name).to_string(),
                value: value.clone(),
            })
            .collect(),
    )
}

fn number(text: &str) -> Value {
    Value::Number(Number::from_checked_text(text.to_string()))
}

#[test]
fn strict_json_reads_every_kind_of_value() {
    let cases = [
        ("null", Value::Null),
        (" true ", Value::Bool(true)),
        ("\t\r\nfalse\n", Value::Bool(false)),
        ("0", number("0")),
        ("-0", number("-0")),
        ("1.5e+10", number("1.5e+10")),
        ("-12.034E-2", number("-12.034E-2")),
        (r#""text""#, Value::from("text")),
        ("[]", Value::Array(vec![])),
        ("{}", object(&[])),
        (
            "[1, [2, {}]]",
            Value::Array(vec![number("1"), Value::Array(vec![number("2"), object(&[])])]),
        ),
        (
            r#"{"b": 1, "a": 2, "b": 3}"#,
            object(&[("b", number("1")), ("a", number("2")), ("b", number("3"))]),
        ),
        (
            r#"{"ключ": "値", "😀": null}"#,
            object(&[("ключ", Value::from("値")), ("😀", Value::Null)]),
        ),
        ("\u{feff}[1]", Value::Array(vec![number("1")])),
    ];
    for (text, expected) in cases {
        assert_eq!(parse(text), Ok(expected.clone()), "{text:?}");
        assert_eq!(parse_with_comments(text), Ok(expected), "{text:?} with comments");
    }
}

#[test]
fn strings_read_every_escape() {
    let cases = [
        (r#""\"\\\/""#, "\"\\/"),
        (r#""\b\f\n\r\t""#, "\u{8}\u{c}\n\r\t"),
        (r#""\u00e9\u00E9""#, "éé"),
        (r#""\ud83d\ude00""#, "😀"),
        (r#""\u0000""#, "\u{0}"),
        ("\"é ✓ 😀\"", "é ✓ 😀"),
        (r#""a\u0022b""#, "a\"b"),
    ];
    for (text, expected) in cases {
        assert_eq!(parse(text), Ok(Value::from(expected)), "{text}");
    }
}

#[test]
fn strict_json_refuses_what_rfc_8259_refuses() {
    let cases = [
        ("", syntax(1, 1, Problem::UnexpectedEnd)),
        ("   ", syntax(1, 4, Problem::UnexpectedEnd)),
        ("[1,]", syntax(1, 3, Problem::TrailingComma)),
        (r#"{"a":1,}"#, syntax(1, 7, Problem::TrailingComma)),
        ("// c\n1", syntax(1, 1, Problem::Comment)),
        ("1 /* c */", syntax(1, 3, Problem::Comment)),
        ("01", syntax(1, 1, Problem::LeadingZero)),
        ("-01", syntax(1, 2, Problem::LeadingZero)),
        ("-", syntax(1, 2, Problem::InvalidNumber)),
        ("1.", syntax(1, 3, Problem::InvalidNumber)),
        ("1.e5", syntax(1, 3, Problem::InvalidNumber)),
        ("1e", syntax(1, 3, Problem::InvalidNumber)),
        ("1e+", syntax(1, 4, Problem::InvalidNumber)),
        (".5", syntax(1, 1, Problem::ExpectedValue)),
        ("+1", syntax(1, 1, Problem::ExpectedValue)),
        ("NaN", syntax(1, 1, Problem::ExpectedValue)),
        ("Infinity", syntax(1, 1, Problem::ExpectedValue)),
        ("tru", syntax(1, 1, Problem::ExpectedValue)),
        ("True", syntax(1, 1, Problem::ExpectedValue)),
        ("\"a\tb\"", syntax(1, 3, Problem::ControlCharacterInString)),
        ("\"a\nb\"", syntax(1, 3, Problem::ControlCharacterInString)),
        (r#""\x""#, syntax(1, 2, Problem::InvalidEscape)),
        (r#""\'""#, syntax(1, 2, Problem::InvalidEscape)),
        (r#""\u12G4""#, syntax(1, 2, Problem::InvalidUnicodeEscape)),
        (r#""\u12""#, syntax(1, 2, Problem::InvalidUnicodeEscape)),
        (r#""\ud800""#, syntax(1, 2, Problem::LoneSurrogate)),
        (r#""\udc00""#, syntax(1, 2, Problem::LoneSurrogate)),
        (r#""\ud800\u0041""#, syntax(1, 2, Problem::LoneSurrogate)),
        (r#""\ud800\ud800""#, syntax(1, 2, Problem::LoneSurrogate)),
        (r#""abc"#, syntax(1, 1, Problem::UnterminatedString)),
        (r#"{"a" 1}"#, syntax(1, 6, Problem::ExpectedColon)),
        ("{1: 2}", syntax(1, 2, Problem::ExpectedMemberName)),
        ("{'a': 1}", syntax(1, 2, Problem::ExpectedMemberName)),
        ("{a: 1}", syntax(1, 2, Problem::ExpectedMemberName)),
        ("[1 2]", syntax(1, 4, Problem::ExpectedCommaOrClosingBracket)),
        (r#"{"a": 1 "b": 2}"#, syntax(1, 9, Problem::ExpectedCommaOrClosingBrace)),
        ("[1", syntax(1, 3, Problem::UnexpectedEnd)),
        (r#"{"a":"#, syntax(1, 6, Problem::UnexpectedEnd)),
        ("{", syntax(1, 2, Problem::UnexpectedEnd)),
        ("1 2", syntax(1, 3, Problem::TextAfterValue)),
        ("1\u{a0}", syntax(1, 2, Problem::TextAfterValue)),
        ("[,]", syntax(1, 2, Problem::ExpectedValue)),
        ("[1,,2]", syntax(1, 4, Problem::ExpectedValue)),
        ("\n\n  x", syntax(3, 3, Problem::ExpectedValue)),
        ("{\r\n  \"a\" 1\r\n}", syntax(2, 7, Problem::ExpectedColon)),
        ("[\"é\" \"ü\"]", syntax(1, 6, Problem::ExpectedCommaOrClosingBracket)),
    ];
    for (text, expected) in cases {
        assert_eq!(parse(text), Err(expected), "{text:?}");
    }
}

#[test]
fn json_with_comments_allows_comments_and_trailing_commas() {
    let text = "// head\n{ /* a */ \"a\" /* b */ : /* c */ 1, // d\n \"b\": [1, 2,], } // tail";
    let expected = object(&[("a", number("1")), ("b", Value::Array(vec![number("1"), number("2")]))]);
    assert_eq!(parse_with_comments(text), Ok(expected));
    let cases = [
        ("/* only */ 5", number("5")),
        ("[1,]", Value::Array(vec![number("1")])),
        ("{\"a\": 1,}", object(&[("a", number("1"))])),
        ("1 // no line break at the end", number("1")),
        ("1 /**/", number("1")),
        ("[1 /* , 2 */]", Value::Array(vec![number("1")])),
        ("\"// not a comment\"", Value::from("// not a comment")),
        ("{\r\n  // a\r\n  \"a\": 1 // b\r\n}\r\n", object(&[("a", number("1"))])),
    ];
    for (text, expected) in cases {
        assert_eq!(parse_with_comments(text), Ok(expected), "{text:?}");
    }
}

#[test]
fn json_with_comments_still_refuses_what_is_not_json() {
    let cases = [
        ("/* open", syntax(1, 1, Problem::UnterminatedComment)),
        ("1 /* open", syntax(1, 3, Problem::UnterminatedComment)),
        ("[,]", syntax(1, 2, Problem::ExpectedValue)),
        ("{,}", syntax(1, 2, Problem::ExpectedMemberName)),
        ("[1,,]", syntax(1, 4, Problem::ExpectedValue)),
        ("/ 1", syntax(1, 1, Problem::ExpectedValue)),
        ("// only a comment", syntax(1, 18, Problem::UnexpectedEnd)),
        ("# not a comment", syntax(1, 1, Problem::ExpectedValue)),
        ("01", syntax(1, 1, Problem::LeadingZero)),
    ];
    for (text, expected) in cases {
        assert_eq!(parse_with_comments(text), Err(expected), "{text:?}");
    }
}

#[test]
fn nesting_past_the_limit_is_an_error_not_a_crash() {
    let nested = |depth: usize| format!("{}{}", "[".repeat(depth), "]".repeat(depth));
    assert!(parse(&nested(NESTING_LIMIT)).is_ok());
    let limit = NESTING_LIMIT;
    assert_eq!(
        parse(&nested(limit + 1)),
        Err(syntax(1, limit + 1, Problem::TooDeep { limit }))
    );
    assert!(matches!(
        parse(&"[{\"a\":".repeat(100_000)),
        Err(Error::Syntax {
            problem: Problem::TooDeep { .. },
            ..
        })
    ));
}

#[test]
fn values_remember_where_they_were_written() {
    let text = "{\"a\": [1, 2,], \"b\": \"x\"}";
    let root = parse_document(text, Dialect::WithComments).unwrap();
    assert_eq!((root.start, root.end), (0, text.len()));
    let NodeKind::Object(members) = &root.kind else {
        panic!("not an object: {root:?}")
    };
    let spans: Vec<_> = members
        .iter()
        .map(|member| (member.start, member.value.start, member.value.end, member.comma))
        .collect();
    assert_eq!(spans, [(1, 6, 13, Some(13)), (15, 20, 23, None)]);
    let NodeKind::Array(items) = &members[0].value.kind else {
        panic!("not an array")
    };
    let item_spans: Vec<_> = items
        .iter()
        .map(|item| (item.start, item.value.end, item.comma))
        .collect();
    assert_eq!(item_spans, [(7, 8, Some(8)), (10, 11, Some(11))]);
}

#[test]
fn blank_texts_and_trivia_are_recognized() {
    assert!(is_blank(""));
    assert!(is_blank(" \r\n\t"));
    assert!(is_blank("\u{feff}\n"));
    assert!(!is_blank("// c"));
    assert_eq!(holds_no_value("  // c\n /* d */ "), Ok(true));
    assert_eq!(holds_no_value("\u{feff}// only"), Ok(true));
    assert_eq!(holds_no_value("// c\n1"), Ok(false));
    assert!(holds_no_value("/* open").is_err());
}
