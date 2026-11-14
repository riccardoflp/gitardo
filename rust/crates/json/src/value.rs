//! A JSON value, with the members of its objects in the order they were written.

use std::fmt;

/// A JSON value. Objects keep their members in the order of the text, names repeated included.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number, as it was written.
    Number(Number),
    /// A string, with its escapes already read.
    String(String),
    /// An array, its items in order.
    Array(Vec<Value>),
    /// An object, its members in the order they were written.
    Object(Vec<Member>),
}

/// One member of an object: a name and its value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    /// The name of the member.
    pub name: String,
    /// The value of the member.
    pub value: Value,
}

/// What kind of value something is, to say so in an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueKind {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool,
    /// A number.
    Number,
    /// A string.
    String,
    /// An array.
    Array,
    /// An object.
    Object,
}

impl fmt::Display for ValueKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            ValueKind::Null => "null",
            ValueKind::Bool => "a boolean",
            ValueKind::Number => "a number",
            ValueKind::String => "a string",
            ValueKind::Array => "an array",
            ValueKind::Object => "an object",
        })
    }
}

impl Value {
    /// What kind of value this is.
    #[must_use]
    pub fn kind(&self) -> ValueKind {
        match self {
            Value::Null => ValueKind::Null,
            Value::Bool(_) => ValueKind::Bool,
            Value::Number(_) => ValueKind::Number,
            Value::String(_) => ValueKind::String,
            Value::Array(_) => ValueKind::Array,
            Value::Object(_) => ValueKind::Object,
        }
    }

    /// Whether this is `null`.
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// The boolean, when this is one.
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(boolean) => Some(*boolean),
            _ => None,
        }
    }

    /// The number, when this is one.
    #[must_use]
    pub fn as_number(&self) -> Option<&Number> {
        match self {
            Value::Number(number) => Some(number),
            _ => None,
        }
    }

    /// The string, when this is one.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(string) => Some(string),
            _ => None,
        }
    }

    /// The items, when this is an array.
    #[must_use]
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    /// The members in order, when this is an object.
    #[must_use]
    pub fn as_object(&self) -> Option<&[Member]> {
        match self {
            Value::Object(members) => Some(members),
            _ => None,
        }
    }

    /// The value of the member with this name, when this is an object that has one. When the name is written more
    /// than once, the last one counts, as it does in editors.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.as_object()?
            .iter()
            .rev()
            .find(|member| member.name == name)
            .map(|member| &member.value)
    }
}

impl From<bool> for Value {
    fn from(boolean: bool) -> Value {
        Value::Bool(boolean)
    }
}

impl From<&str> for Value {
    fn from(string: &str) -> Value {
        Value::String(string.to_string())
    }
}

impl From<String> for Value {
    fn from(string: String) -> Value {
        Value::String(string)
    }
}

impl From<Number> for Value {
    fn from(number: Number) -> Value {
        Value::Number(number)
    }
}

impl From<i64> for Value {
    fn from(number: i64) -> Value {
        Value::Number(Number::from(number))
    }
}

impl From<u64> for Value {
    fn from(number: u64) -> Value {
        Value::Number(Number::from(number))
    }
}

/// A JSON number, kept as the text it was written with, so that writing it again gives back the same text and no
/// precision is lost. Two numbers are equal when they are written the same way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Number {
    text: String,
}

/// How many zeros an exponent may add to a whole number: more than any 64-bit integer has digits.
const LARGEST_EXPONENT_SHIFT: usize = 20;

impl Number {
    /// A number from text the parser has already checked against the grammar of JSON.
    pub(crate) fn from_checked_text(text: String) -> Number {
        Number { text }
    }

    /// The number of a floating point value, written in its shortest exact form. Not a number and the infinities have
    /// no JSON form, so they give nothing.
    #[must_use]
    pub fn from_f64(number: f64) -> Option<Number> {
        // The debug form of a finite f64 is the shortest text that reads back the same, and is always valid JSON.
        number.is_finite().then(|| Number {
            text: format!("{number:?}"),
        })
    }

    /// The number as it is written.
    #[must_use]
    pub fn as_text(&self) -> &str {
        &self.text
    }

    /// The number as the nearest floating point value, or nothing when it is too large for one.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        self.text.parse::<f64>().ok().filter(|number| number.is_finite())
    }

    /// The number as a signed integer, when it is a whole number that fits, however it is written: `1000`, `1e3` and
    /// `1000.0` all give 1000.
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        whole_number_digits(&self.text)?.parse().ok()
    }

    /// The number as an unsigned integer, when it is a whole number from zero up that fits, however it is written.
    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        whole_number_digits(&self.text)?.parse().ok()
    }
}

impl From<i64> for Number {
    fn from(number: i64) -> Number {
        Number {
            text: number.to_string(),
        }
    }
}

impl From<u64> for Number {
    fn from(number: u64) -> Number {
        Number {
            text: number.to_string(),
        }
    }
}

impl fmt::Display for Number {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

/// The digits of a JSON number written as a plain integer, with a minus sign when it is below zero, or nothing when
/// the number has a fraction or is far too large for an integer.
fn whole_number_digits(text: &str) -> Option<String> {
    let (is_negative, unsigned) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (mantissa, exponent) = match unsigned.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, exponent.parse::<i64>().ok()?),
        None => (unsigned, 0),
    };
    let (integer, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = format!("{integer}{fraction}");
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return Some("0".to_string());
    }
    let shift = exponent.checked_sub(i64::try_from(fraction.len()).ok()?)?;
    let whole = if shift >= 0 {
        let zeros = usize::try_from(shift)
            .ok()
            .filter(|zeros| *zeros <= LARGEST_EXPONENT_SHIFT)?;
        format!("{digits}{}", "0".repeat(zeros))
    } else {
        let dropped = usize::try_from(shift.unsigned_abs()).ok()?;
        let kept = digits.len().checked_sub(dropped)?;
        let (whole, rest) = digits.split_at_checked(kept)?;
        if !rest.bytes().all(|digit| digit == b'0') {
            return None;
        }
        whole.to_string()
    };
    Some(if is_negative { format!("-{whole}") } else { whole })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(text: &str) -> Number {
        Number::from_checked_text(text.to_string())
    }

    #[test]
    fn numbers_read_as_integers_when_they_are_whole() {
        let cases: [(&str, Option<i64>, Option<u64>); 14] = [
            ("0", Some(0), Some(0)),
            ("-0", Some(0), Some(0)),
            ("42", Some(42), Some(42)),
            ("-42", Some(-42), None),
            ("1e3", Some(1000), Some(1000)),
            ("1000.0", Some(1000), Some(1000)),
            ("1.5e1", Some(15), Some(15)),
            ("1500e-2", Some(15), Some(15)),
            ("1.5", None, None),
            ("15e-2", None, None),
            ("9223372036854775807", Some(i64::MAX), Some(9_223_372_036_854_775_807)),
            ("18446744073709551615", None, Some(u64::MAX)),
            ("1e400", None, None),
            ("0.000e999999", Some(0), Some(0)),
        ];
        for (text, signed, unsigned) in cases {
            assert_eq!(number(text).as_i64(), signed, "{text} as i64");
            assert_eq!(number(text).as_u64(), unsigned, "{text} as u64");
        }
    }

    #[test]
    fn numbers_read_as_floating_point_when_they_fit() {
        let cases: [(&str, Option<f64>); 5] = [
            ("0.5", Some(0.5)),
            ("-1e2", Some(-100.0)),
            ("1E+2", Some(100.0)),
            ("1e400", None),
            ("12", Some(12.0)),
        ];
        for (text, expected) in cases {
            assert_eq!(number(text).as_f64(), expected, "{text}");
        }
    }

    #[test]
    fn floating_point_values_are_written_in_their_shortest_form() {
        let cases: [(f64, Option<&str>); 6] = [
            (1.0, Some("1.0")),
            (0.1, Some("0.1")),
            (-2.5e-7, Some("-2.5e-7")),
            (1e300, Some("1e300")),
            (f64::NAN, None),
            (f64::INFINITY, None),
        ];
        for (value, expected) in cases {
            assert_eq!(
                Number::from_f64(value).as_ref().map(Number::as_text),
                expected,
                "{value}"
            );
        }
    }

    #[test]
    fn accessors_give_the_value_only_for_its_kind() {
        let object = Value::Object(vec![
            Member {
                name: "a".into(),
                value: Value::from(1_i64),
            },
            Member {
                name: "b".into(),
                value: Value::from("text"),
            },
            Member {
                name: "a".into(),
                value: Value::from(true),
            },
        ]);
        assert_eq!(object.get("a"), Some(&Value::Bool(true)));
        assert_eq!(object.get("b").and_then(Value::as_str), Some("text"));
        assert_eq!(object.get("c"), None);
        assert_eq!(object.as_object().map(<[Member]>::len), Some(3));
        assert_eq!(object.as_array(), None);
        assert_eq!(Value::Null.get("a"), None);
        assert!(Value::Null.is_null());
        assert_eq!(Value::from(false).as_bool(), Some(false));
        assert_eq!(Value::Array(vec![Value::Null]).as_array(), Some(&[Value::Null][..]));
        assert_eq!(Value::from(7_u64).as_number().and_then(Number::as_u64), Some(7));
        assert_eq!(Value::from("x").kind(), ValueKind::String);
    }
}
