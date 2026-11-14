//! Writing values as JSON text: indented for people, compact for the network, and in the style of a file being edited.

use crate::value::{Member, Value};

/// The value as indented JSON for a person to read: four spaces for each level, `"name": value`, one item or member
/// on each line, and a line feed at the end. Members keep their order, so the same value always gives the same text.
#[must_use]
pub fn to_pretty_text(value: &Value) -> String {
    let lines = Lines {
        unit: "    ",
        newline: "\n",
        base: "",
    };
    let mut text = String::new();
    write_value(value, Style::Lines(lines), &mut text);
    text.push('\n');
    text
}

/// The value as JSON with no white space at all, such as the body of a request to a web service.
#[must_use]
pub fn to_compact_text(value: &Value) -> String {
    let mut text = String::new();
    write_value(value, Style::Compact, &mut text);
    text
}

/// How the arrays and objects of a value are laid out.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Style<'a> {
    /// No white space: `{"a":[1,2]}`.
    Compact,
    /// On one line, for a person: `{"a": [1, 2]}`.
    Inline,
    /// One item or member on each line.
    Lines(Lines<'a>),
}

/// How a value written on several lines is indented.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Lines<'a> {
    /// The indentation of one level: some spaces or a tab.
    pub(crate) unit: &'a str,
    /// What ends a line: a line feed, or a carriage return and a line feed.
    pub(crate) newline: &'a str,
    /// The indentation of the line the value starts on, which every following line starts with too.
    pub(crate) base: &'a str,
}

/// Writes a value at the end of a text, in a style.
pub(crate) fn write_value(value: &Value, style: Style<'_>, text: &mut String) {
    Writer { style, text }.value(value, 0);
}

/// Writes the member of an object, its name, its colon and its value, at the end of a text, in a style.
pub(crate) fn write_member(name: &str, value: &Value, style: Style<'_>, text: &mut String) {
    let mut writer = Writer { style, text };
    writer.member(name, value, 0);
}

/// Writes a string between double quotes, escaping what JSON needs escaped: quotes, backslashes and control
/// characters, the common ones with their short escapes and the others as `\u00XX`.
pub(crate) fn write_string(string: &str, text: &mut String) {
    text.push('"');
    for character in string.chars() {
        match character {
            '"' => text.push_str("\\\""),
            '\\' => text.push_str("\\\\"),
            '\n' => text.push_str("\\n"),
            '\r' => text.push_str("\\r"),
            '\t' => text.push_str("\\t"),
            '\u{8}' => text.push_str("\\b"),
            '\u{c}' => text.push_str("\\f"),
            control if u32::from(control) < 0x20 => {
                let code = u32::from(control);
                text.push_str("\\u00");
                text.extend([code >> 4, code & 0xf].map(|digit| char::from_digit(digit, 16).unwrap_or('0')));
            }
            other => text.push(other),
        }
    }
    text.push('"');
}

/// Writes one value, and what is in it, at the end of a text.
struct Writer<'a, 'b> {
    style: Style<'a>,
    text: &'b mut String,
}

impl Writer<'_, '_> {
    fn value(&mut self, value: &Value, depth: usize) {
        match value {
            Value::Null => self.text.push_str("null"),
            Value::Bool(true) => self.text.push_str("true"),
            Value::Bool(false) => self.text.push_str("false"),
            Value::Number(number) => self.text.push_str(number.as_text()),
            Value::String(string) => write_string(string, self.text),
            Value::Array(items) => {
                self.sequence(items.len(), ('[', ']'), depth, |writer, index| {
                    if let Some(item) = items.get(index) {
                        writer.value(item, depth + 1);
                    }
                });
            }
            Value::Object(members) => {
                self.sequence(members.len(), ('{', '}'), depth, |writer, index| {
                    if let Some(Member { name, value }) = members.get(index) {
                        writer.member(name, value, depth + 1);
                    }
                });
            }
        }
    }

    fn member(&mut self, name: &str, value: &Value, depth: usize) {
        write_string(name, self.text);
        self.text.push_str(if matches!(self.style, Style::Compact) {
            ":"
        } else {
            ": "
        });
        self.value(value, depth);
    }

    /// Writes the brackets of an array or the braces of an object, and its entries between them, separated as the
    /// style says. An empty one is always written as its two brackets or braces together.
    fn sequence(
        &mut self,
        length: usize,
        brackets: (char, char),
        depth: usize,
        mut entry: impl FnMut(&mut Self, usize),
    ) {
        let (open, close) = brackets;
        self.text.push(open);
        for index in 0..length {
            if index > 0 {
                self.text.push(',');
            }
            match self.style {
                Style::Compact => {}
                Style::Inline if index == 0 => {}
                Style::Inline => self.text.push(' '),
                Style::Lines(lines) => self.line_start(lines, depth + 1),
            }
            entry(self, index);
        }
        if let (Style::Lines(lines), true) = (self.style, length > 0) {
            self.line_start(lines, depth);
        }
        self.text.push(close);
    }

    fn line_start(&mut self, lines: Lines<'_>, depth: usize) {
        self.text.push_str(lines.newline);
        self.text.push_str(lines.base);
        for _ in 0..depth {
            self.text.push_str(lines.unit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Number;

    fn sample() -> Value {
        Value::Object(vec![
            Member {
                name: "name".into(),
                value: Value::from("Gitardo \"Desktop\""),
            },
            Member {
                name: "size".into(),
                value: Value::Number(Number::from(14_i64)),
            },
            Member {
                name: "list".into(),
                value: Value::Array(vec![Value::Null, Value::from(true), Value::Array(vec![])]),
            },
            Member {
                name: "empty".into(),
                value: Value::Object(vec![]),
            },
            Member {
                name: "nested".into(),
                value: Value::Object(vec![Member {
                    name: "a\u{1}".into(),
                    value: Value::from(false),
                }]),
            },
        ])
    }

    #[test]
    fn pretty_text_indents_four_spaces_and_ends_with_a_line_feed() {
        let expected = concat!(
            "{\n",
            "    \"name\": \"Gitardo \\\"Desktop\\\"\",\n",
            "    \"size\": 14,\n",
            "    \"list\": [\n",
            "        null,\n",
            "        true,\n",
            "        []\n",
            "    ],\n",
            "    \"empty\": {},\n",
            "    \"nested\": {\n",
            "        \"a\\u0001\": false\n",
            "    }\n",
            "}\n",
        );
        assert_eq!(to_pretty_text(&sample()), expected);
    }

    #[test]
    fn compact_text_has_no_white_space() {
        let expected =
            r#"{"name":"Gitardo \"Desktop\"","size":14,"list":[null,true,[]],"empty":{},"nested":{"a\u0001":false}}"#;
        assert_eq!(to_compact_text(&sample()), expected);
    }

    #[test]
    fn scalars_are_written_alone() {
        let cases = [
            (Value::Null, "null"),
            (Value::from(false), "false"),
            (Value::from(-3_i64), "-3"),
            (Value::from("é ✓ 😀"), "\"é ✓ 😀\""),
            (Value::from("a/b"), "\"a/b\""),
        ];
        for (value, expected) in cases {
            assert_eq!(to_compact_text(&value), expected);
            assert_eq!(to_pretty_text(&value), format!("{expected}\n"));
        }
    }

    #[test]
    fn strings_escape_quotes_backslashes_and_control_characters() {
        let cases = [
            ("plain", r#""plain""#),
            ("quote \" and \\", r#""quote \" and \\""#),
            ("lines\r\n\ttab", r#""lines\r\n\ttab""#),
            ("\u{8}\u{c}\u{0}\u{1f}\u{7f}", "\"\\b\\f\\u0000\\u001f\u{7f}\""),
        ];
        for (string, expected) in cases {
            let mut text = String::new();
            write_string(string, &mut text);
            assert_eq!(text, expected);
        }
    }

    #[test]
    fn inline_and_indented_styles_follow_the_file() {
        let value = Value::Object(vec![Member {
            name: "a".into(),
            value: Value::Array(vec![Value::from(1_i64)]),
        }]);
        let mut inline = String::new();
        write_value(&value, Style::Inline, &mut inline);
        assert_eq!(inline, r#"{"a": [1]}"#);
        let mut tabs = String::new();
        let lines = Lines {
            unit: "\t",
            newline: "\r\n",
            base: "\t",
        };
        write_member("key", &value, Style::Lines(lines), &mut tabs);
        assert_eq!(tabs, "\"key\": {\r\n\t\t\"a\": [\r\n\t\t\t1\r\n\t\t]\r\n\t}");
    }
}
