//! Reading the strings and numbers of a JSON text, as RFC 8259 writes them.

use super::Parser;
use crate::error::{Error, Problem};
use crate::value::Number;

impl Parser<'_> {
    /// Reads a string from its opening quote to its closing one, its escapes turned into the characters they mean.
    pub(super) fn parse_string(&mut self) -> Result<String, Error> {
        let opening = self.position;
        self.position += 1;
        let mut string = String::new();
        loop {
            let run_start = self.position;
            while self
                .peek()
                .is_some_and(|byte| byte != b'"' && byte != b'\\' && byte >= 0x20)
            {
                self.position += 1;
            }
            // The run stops at an ASCII byte or at the end, so both of its ends are boundaries of characters.
            string.push_str(self.text.get(run_start..self.position).unwrap_or_default());
            match self.peek() {
                Some(b'"') => {
                    self.position += 1;
                    return Ok(string);
                }
                Some(b'\\') => string.push(self.parse_escape()?),
                Some(_) => return Err(self.error(Problem::ControlCharacterInString)),
                None => return Err(self.error_at(opening, Problem::UnterminatedString)),
            }
        }
    }

    /// Reads an escape from its backslash, and gives the character it means.
    fn parse_escape(&mut self) -> Result<char, Error> {
        let backslash = self.position;
        let character = match self.peek_at(1) {
            Some(b'"') => '"',
            Some(b'\\') => '\\',
            Some(b'/') => '/',
            Some(b'b') => '\u{8}',
            Some(b'f') => '\u{c}',
            Some(b'n') => '\n',
            Some(b'r') => '\r',
            Some(b't') => '\t',
            Some(b'u') => return self.parse_unicode_escape(),
            _ => return Err(self.error_at(backslash, Problem::InvalidEscape)),
        };
        self.position += 2;
        Ok(character)
    }

    /// Reads a `\u` escape, and the one that must follow it when it is the first half of a surrogate pair.
    fn parse_unicode_escape(&mut self) -> Result<char, Error> {
        let backslash = self.position;
        let first = self.parse_hexadecimal_unit()?;
        let code = match first {
            0xD800..=0xDBFF => {
                let is_escape_next = self.peek() == Some(b'\\') && self.peek_at(1) == Some(b'u');
                let second = if is_escape_next {
                    self.parse_hexadecimal_unit()?
                } else {
                    0
                };
                if !(0xDC00..=0xDFFF).contains(&second) {
                    return Err(self.error_at(backslash, Problem::LoneSurrogate));
                }
                0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
            }
            0xDC00..=0xDFFF => return Err(self.error_at(backslash, Problem::LoneSurrogate)),
            _ => first,
        };
        char::from_u32(code).ok_or_else(|| self.error_at(backslash, Problem::LoneSurrogate))
    }

    /// Reads `\u` and the four hexadecimal digits after it.
    fn parse_hexadecimal_unit(&mut self) -> Result<u32, Error> {
        let digits = self.text.get(self.position + 2..self.position + 6).unwrap_or_default();
        let is_hexadecimal = digits.len() == 4 && digits.bytes().all(|byte| byte.is_ascii_hexdigit());
        let unit = u32::from_str_radix(digits, 16).ok().filter(|_| is_hexadecimal);
        let unit = unit.ok_or_else(|| self.error(Problem::InvalidUnicodeEscape))?;
        self.position += 6;
        Ok(unit)
    }

    /// Reads a number as RFC 8259 writes it: a minus sign, an integer with no leading zero, a fraction, an exponent.
    pub(super) fn parse_number(&mut self) -> Result<Number, Error> {
        let start = self.position;
        if self.peek() == Some(b'-') {
            self.position += 1;
        }
        if self.peek() == Some(b'0') && self.peek_at(1).is_some_and(|byte| byte.is_ascii_digit()) {
            return Err(self.error(Problem::LeadingZero));
        }
        self.skip_digits()?;
        if self.peek() == Some(b'.') {
            self.position += 1;
            self.skip_digits()?;
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            self.skip_digits()?;
        }
        let text = self.text.get(start..self.position).unwrap_or_default();
        Ok(Number::from_checked_text(text.to_string()))
    }

    /// Moves past one digit or more.
    fn skip_digits(&mut self) -> Result<(), Error> {
        if !self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            return Err(self.error(Problem::InvalidNumber));
        }
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.position += 1;
        }
        Ok(())
    }
}
