//! A reader of the TOML that Cargo writes and that the manifests of this workspace use: tables, arrays of tables,
//! dotted keys, strings, booleans, arrays and inline tables. Written from the TOML 1.0 specification; it reads only,
//! and keeps what it does not need (numbers, dates) as the bare text.

/// A value of a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// A string, with its escapes resolved.
    Text(String),
    /// `true` or `false`.
    Boolean(bool),
    /// Anything else written without quotes, such as a number or a date, as written.
    Bare(String),
    /// An array of values.
    Array(Vec<Value>),
    /// An inline table, its keys in the order written; a dotted key is joined with dots.
    Table(Vec<(String, Value)>),
}

impl Value {
    /// The text of a string value.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(text) => Some(text),
            Value::Boolean(_) | Value::Bare(_) | Value::Array(_) | Value::Table(_) => None,
        }
    }

    /// The value of a key of an inline table.
    pub fn member(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Table(members) => members.iter().find(|(name, _)| name == key).map(|(_, value)| value),
            Value::Text(_) | Value::Boolean(_) | Value::Bare(_) | Value::Array(_) => None,
        }
    }
}

/// One key and its value, with the table it is in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The name of the table, in segments: `["target", "cfg(windows)", "dependencies"]`.
    pub table: Vec<String>,
    /// Which table header this entry follows, counting from 1; 0 before the first. Tells the elements of an array of
    /// tables apart.
    pub group: usize,
    /// The key, in segments: `serde.workspace` is `["serde", "workspace"]`.
    pub key: Vec<String>,
    /// The value.
    pub value: Value,
    /// The line of the key, from 1.
    pub line: usize,
}

/// Where the text is not TOML this reader understands, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Malformed {
    /// The line, from 1.
    pub line: usize,
    /// What is wrong there.
    pub message: String,
}

/// The entries of a TOML document, in the order written.
pub fn parse(text: &str) -> Result<Vec<Entry>, Malformed> {
    let mut reader = Reader {
        characters: text.chars().collect(),
        position: 0,
        line: 1,
    };
    reader.document().map_err(|message| Malformed {
        line: reader.line,
        message,
    })
}

struct Reader {
    characters: Vec<char>,
    position: usize,
    line: usize,
}

impl Reader {
    fn document(&mut self) -> Result<Vec<Entry>, String> {
        let mut entries = Vec::new();
        let mut table = Vec::new();
        let mut group = 0;
        loop {
            self.skip_blank();
            match self.peek() {
                None => return Ok(entries),
                Some('[') => {
                    table = self.header()?;
                    group += 1;
                }
                Some(_) => {
                    let line = self.line;
                    let (key, value) = self.key_value()?;
                    entries.push(Entry {
                        table: table.clone(),
                        group,
                        key,
                        value,
                        line,
                    });
                }
            }
            self.end_of_line()?;
        }
    }

    fn header(&mut self) -> Result<Vec<String>, String> {
        self.expect('[')?;
        let is_array = self.peek() == Some('[');
        if is_array {
            self.expect('[')?;
        }
        self.skip_spaces();
        let name = self.key()?;
        self.expect(']')?;
        if is_array {
            self.expect(']')?;
        }
        Ok(name)
    }

    fn key_value(&mut self) -> Result<(Vec<String>, Value), String> {
        let key = self.key()?;
        self.expect('=')?;
        self.skip_spaces();
        let value = self.value()?;
        Ok((key, value))
    }

    fn key(&mut self) -> Result<Vec<String>, String> {
        let mut segments = vec![self.key_segment()?];
        self.skip_spaces();
        while self.peek() == Some('.') {
            self.position += 1;
            self.skip_spaces();
            segments.push(self.key_segment()?);
            self.skip_spaces();
        }
        Ok(segments)
    }

    fn key_segment(&mut self) -> Result<String, String> {
        match self.peek() {
            Some('"') => self.basic_string(),
            Some('\'') => self.literal_string(),
            _ => {
                let segment =
                    self.take_while(|character| character.is_ascii_alphanumeric() || "_-".contains(character));
                if segment.is_empty() {
                    Err(String::from("A key was expected here."))
                } else {
                    Ok(segment)
                }
            }
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        match self.peek() {
            Some('"') => self.basic_string().map(Value::Text),
            Some('\'') => self.literal_string().map(Value::Text),
            Some('[') => self.array(),
            Some('{') => self.inline_table(),
            _ => {
                let bare = self.take_while(|character| !",]}#\r\n".contains(character));
                let bare = bare.trim_end().to_owned();
                match bare.as_str() {
                    "" => Err(String::from("A value was expected here.")),
                    "true" => Ok(Value::Boolean(true)),
                    "false" => Ok(Value::Boolean(false)),
                    _ => Ok(Value::Bare(bare)),
                }
            }
        }
    }

    fn array(&mut self) -> Result<Value, String> {
        self.expect('[')?;
        let mut values = Vec::new();
        loop {
            self.skip_blank();
            if self.peek() == Some(']') {
                self.position += 1;
                return Ok(Value::Array(values));
            }
            values.push(self.value()?);
            self.skip_blank();
            match self.advance() {
                Some(',') => {}
                Some(']') => return Ok(Value::Array(values)),
                _ => return Err(String::from("A comma or the end of the array was expected here.")),
            }
        }
    }

    fn inline_table(&mut self) -> Result<Value, String> {
        self.expect('{')?;
        let mut members = Vec::new();
        loop {
            self.skip_spaces();
            if self.peek() == Some('}') {
                self.position += 1;
                return Ok(Value::Table(members));
            }
            let (key, value) = self.key_value()?;
            members.push((key.join("."), value));
            self.skip_spaces();
            match self.advance() {
                Some(',') => {}
                Some('}') => return Ok(Value::Table(members)),
                _ => {
                    return Err(String::from(
                        "A comma or the end of the inline table was expected here.",
                    ));
                }
            }
        }
    }

    fn basic_string(&mut self) -> Result<String, String> {
        if self.starts_with("\"\"\"") {
            return self.multiline_string("\"\"\"");
        }
        self.expect('"')?;
        let mut text = String::new();
        loop {
            let character = self
                .peek()
                .filter(|character| *character != '\n')
                .ok_or_else(|| String::from("The string is not closed on its line."))?;
            self.position += 1;
            match character {
                '"' => return Ok(text),
                '\\' => text.push(self.escape()?),
                _ => text.push(character),
            }
        }
    }

    fn literal_string(&mut self) -> Result<String, String> {
        if self.starts_with("'''") {
            return self.multiline_string("'''");
        }
        self.expect('\'')?;
        let text = self.take_while(|character| character != '\'' && character != '\n');
        self.expect('\'')?;
        Ok(text)
    }

    fn multiline_string(&mut self, delimiter: &str) -> Result<String, String> {
        self.position += delimiter.chars().count();
        let mut text = String::new();
        while !self.starts_with(delimiter) {
            match self.advance() {
                None => return Err(String::from("The multi-line string is not closed.")),
                Some('\\') if delimiter == "\"\"\"" => text.push(self.escape()?),
                Some(character) => text.push(character),
            }
        }
        self.position += delimiter.chars().count();
        Ok(text.strip_prefix('\n').map(str::to_owned).unwrap_or(text))
    }

    fn escape(&mut self) -> Result<char, String> {
        let unicode_length = match self.advance() {
            Some('b') => return Ok('\u{8}'),
            Some('t') => return Ok('\t'),
            Some('n') => return Ok('\n'),
            Some('f') => return Ok('\u{c}'),
            Some('r') => return Ok('\r'),
            Some('"') => return Ok('"'),
            Some('\\') => return Ok('\\'),
            Some('u') => 4,
            Some('U') => 8,
            _ => return Err(String::from("The string has an escape TOML does not know.")),
        };
        let digits: String = (0..unicode_length).filter_map(|_| self.advance()).collect();
        u32::from_str_radix(&digits, 16)
            .ok()
            .and_then(char::from_u32)
            .ok_or_else(|| String::from("The string has an escape that is not a character."))
    }

    fn end_of_line(&mut self) -> Result<(), String> {
        self.skip_spaces();
        match self.peek() {
            None | Some('\n' | '\r' | '#') => Ok(()),
            Some(_) => Err(String::from("The line goes on after its value.")),
        }
    }

    fn skip_blank(&mut self) {
        loop {
            match self.peek() {
                Some(' ' | '\t' | '\r' | '\n') => {
                    let _ = self.advance();
                }
                Some('#') => self.skip_comment(),
                _ => return,
            }
        }
    }

    fn skip_spaces(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.position += 1;
        }
        if self.peek() == Some('#') {
            self.skip_comment();
        }
    }

    fn skip_comment(&mut self) {
        while !matches!(self.peek(), None | Some('\n')) {
            self.position += 1;
        }
    }

    fn take_while(&mut self, wanted: impl Fn(char) -> bool) -> String {
        let mut taken = String::new();
        while let Some(character) = self.peek().filter(|character| wanted(*character)) {
            taken.push(character);
            self.position += 1;
        }
        taken
    }

    fn starts_with(&self, text: &str) -> bool {
        text.chars()
            .enumerate()
            .all(|(offset, character)| self.characters.get(self.position + offset) == Some(&character))
    }

    fn expect(&mut self, wanted: char) -> Result<(), String> {
        self.skip_spaces();
        if self.peek() == Some(wanted) {
            self.position += 1;
            Ok(())
        } else {
            Err(format!("`{wanted}` was expected here."))
        }
    }

    fn peek(&self) -> Option<char> {
        self.characters.get(self.position).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.position += 1;
        if character == '\n' {
            self.line += 1;
        }
        Some(character)
    }
}

#[cfg(test)]
mod tests;
