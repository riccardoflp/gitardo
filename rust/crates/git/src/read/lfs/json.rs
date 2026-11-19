//! A small reader of the JSON Git LFS prints for its locks: objects, arrays, strings, numbers, `true`, `false` and
//! `null`. The crate depends on `model` and `text` only (rule R1.3), so it does not take the reader of `crates/json`.

/// A value of JSON.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Json {
    /// `null`, `true` or `false`, or a number, as written.
    Word(String),
    /// A string, its escapes read.
    Text(String),
    /// An array.
    Array(Vec<Json>),
    /// An object, its members in order.
    Object(Vec<(String, Json)>),
}

impl Json {
    /// The member of an object by its name.
    pub(super) fn get(&self, name: &str) -> Option<&Json> {
        match self {
            Json::Object(members) => members.iter().find(|(found, _)| found == name).map(|(_, value)| value),
            _ => None,
        }
    }

    /// The text of a string, or of a number or a word as written.
    pub(super) fn text(&self) -> Option<&str> {
        match self {
            Json::Text(text) | Json::Word(text) => Some(text),
            _ => None,
        }
    }

    /// The values of an array; none for anything else.
    pub(super) fn items(&self) -> &[Json] {
        match self {
            Json::Array(items) => items,
            _ => &[],
        }
    }
}

/// The value a text holds, spaces around it left out; none when it is not JSON.
pub(super) fn parse(text: &str) -> Option<Json> {
    let mut reader = Reader {
        characters: text.chars().collect(),
        at: 0,
    };
    let value = reader.value()?;
    reader.spaces();
    (reader.at == reader.characters.len()).then_some(value)
}

/// Where a text is being read.
struct Reader {
    /// Its characters.
    characters: Vec<char>,
    /// The place of the next one.
    at: usize,
}

impl Reader {
    /// The next character, not taken.
    fn peek(&self) -> Option<char> {
        self.characters.get(self.at).copied()
    }

    /// The next character, taken.
    fn next(&mut self) -> Option<char> {
        let found = self.peek()?;
        self.at += 1;
        Some(found)
    }

    /// Takes the spaces before what comes.
    fn spaces(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.at += 1;
        }
    }

    /// A value.
    fn value(&mut self) -> Option<Json> {
        self.spaces();
        match self.peek()? {
            '{' => self.object(),
            '[' => self.array(),
            '"' => self.string().map(Json::Text),
            _ => self.word(),
        }
    }

    /// `null`, `true`, `false` or a number, as written.
    fn word(&mut self) -> Option<Json> {
        let start = self.at;
        while self
            .peek()
            .is_some_and(|found| found.is_ascii_alphanumeric() || matches!(found, '-' | '+' | '.'))
        {
            self.at += 1;
        }
        let word: String = self.characters.get(start..self.at)?.iter().collect();
        (!word.is_empty()).then_some(Json::Word(word))
    }

    /// A string, from its opening quote.
    fn string(&mut self) -> Option<String> {
        let _quote = self.next()?;
        let mut text = String::new();
        loop {
            match self.next()? {
                '"' => return Some(text),
                '\\' => text.push(self.escape()?),
                found => text.push(found),
            }
        }
    }

    /// The character an escape stands for, after its backslash.
    fn escape(&mut self) -> Option<char> {
        Some(match self.next()? {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            'b' => '\u{8}',
            'f' => '\u{c}',
            'u' => {
                let digits: String = (0..4).filter_map(|_| self.next()).collect();
                char::from_u32(u32::from_str_radix(&digits, 16).ok()?).unwrap_or('\u{fffd}')
            }
            other => other,
        })
    }

    /// An array, from its bracket.
    fn array(&mut self) -> Option<Json> {
        let _bracket = self.next()?;
        let mut items = Vec::new();
        loop {
            self.spaces();
            match self.peek()? {
                ']' => {
                    self.at += 1;
                    return Some(Json::Array(items));
                }
                ',' => self.at += 1,
                _ => items.push(self.value()?),
            }
        }
    }

    /// An object, from its brace.
    fn object(&mut self) -> Option<Json> {
        let _brace = self.next()?;
        let mut members = Vec::new();
        loop {
            self.spaces();
            match self.peek()? {
                '}' => {
                    self.at += 1;
                    return Some(Json::Object(members));
                }
                ',' => self.at += 1,
                '"' => {
                    let name = self.string()?;
                    self.spaces();
                    (self.next()? == ':').then_some(())?;
                    members.push((name, self.value()?));
                }
                _ => return None,
            }
        }
    }
}
