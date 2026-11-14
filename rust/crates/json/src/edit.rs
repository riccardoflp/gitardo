//! Changing one value in the text of a file of JSON with comments, keeping every other byte as it was: comments,
//! order, blank lines and indentation.

use crate::error::{Error, Key};
use crate::layout::{
    Conventions, DEFAULT_UNIT, Shape, is_at_line_start, is_white_space, line_indent, line_start, newline_length_at,
    newline_of, same_line_trivia_end, shape_of, skip_spaces, skip_spaces_back,
};
use crate::parse::{BYTE_ORDER_MARK, Dialect, Entry, Node, NodeKind, holds_no_value, is_blank, parse_document};
use crate::value::{Member, Value, ValueKind};
use crate::write::{Lines, Style, write_member, write_value};

/// The text of a file of JSON with comments with one value set: the one the path of keys leads to, from the top of
/// the document. Only that value changes; the rest of the text stays byte for byte.
///
/// A member that is missing is added at the end of its object, indented as the file indents, with the comma the file
/// needs; missing objects on the way are added too. An index one past the end of an array adds an item. An empty
/// path sets the whole document, and a text with no value at all becomes an object that holds the path.
///
/// # Errors
///
/// [`Error::Syntax`] when the text is not valid JSON with comments; [`Error::NotAContainer`] when the path goes
/// through a value that is not the object or array its next key needs; [`Error::IndexOutOfRange`] when an index is
/// more than one past the end of its array.
pub fn set_value(text: &str, path: &[Key], value: &Value) -> Result<String, Error> {
    if is_blank(text) {
        // The white space goes, and a byte order mark stays, so that the file keeps its encoding.
        let mark = if text.starts_with(BYTE_ORDER_MARK) {
            BYTE_ORDER_MARK
        } else {
            ""
        };
        return Ok(format!("{mark}{}", new_document(text, path, value)?));
    }
    if holds_no_value(text)? {
        // A file of comments only keeps them, and the new object follows them.
        let mut result = text.to_string();
        if !result.ends_with('\n') {
            result.push_str(newline_of(text));
        }
        result.push_str(&new_document(text, path, value)?);
        return Ok(result);
    }
    let root = parse_document(text, Dialect::WithComments)?;
    let file = File {
        text,
        conventions: Conventions::of(text, &root),
        root_start: root.start,
    };
    let splices = match locate(&root, path)? {
        Location::Root => vec![file.replace(&root, &root, value)],
        Location::Existing { container, entry, .. } => vec![file.replace(container, &entry.value, value)],
        Location::Missing {
            container,
            entries,
            depth,
        } => file.insert(container, entries, &NewEntry { path, depth, value })?,
    };
    Ok(apply(text, splices))
}

/// The text of a file of JSON with comments with one value removed: the member or item the path of keys leads to.
/// Its line goes with it when it is alone on it, and so does a comment after it on that line; the commas around it
/// are fixed, and every other byte, the comments of the other members included, stays as it was. A path that leads
/// to nothing gives the text back as it is.
///
/// # Errors
///
/// [`Error::Syntax`] when the text is not valid JSON with comments; [`Error::NotAContainer`] when the path goes
/// through a value that is not the object or array its next key needs; [`Error::EmptyPath`] when the path is empty.
pub fn remove_value(text: &str, path: &[Key]) -> Result<String, Error> {
    if path.is_empty() {
        return Err(Error::EmptyPath);
    }
    if holds_no_value(text)? {
        return Ok(text.to_string());
    }
    let root = parse_document(text, Dialect::WithComments)?;
    let splices = match locate(&root, path)? {
        Location::Existing {
            container,
            entries,
            index,
            entry,
        } => removal(text, container, entries, (index, entry)),
        Location::Root | Location::Missing { .. } => Vec::new(),
    };
    Ok(apply(text, splices))
}

/// One change to a text: the bytes from `start` to `end` replaced.
#[derive(Debug)]
struct Splice {
    start: usize,
    end: usize,
    replacement: String,
}

/// The text with its splices made. Splices never overlap, and each is made at its place in the original text.
fn apply(text: &str, mut splices: Vec<Splice>) -> String {
    splices.sort_by_key(|splice| splice.start);
    let mut result = String::with_capacity(text.len());
    let mut copied = 0;
    for splice in splices {
        result.push_str(text.get(copied..splice.start).unwrap_or_default());
        result.push_str(&splice.replacement);
        copied = splice.end;
    }
    result.push_str(text.get(copied..).unwrap_or_default());
    result
}

/// The text of a new document that holds the value at the path, for a file that has none yet.
fn new_document(text: &str, path: &[Key], value: &Value) -> Result<String, Error> {
    let newline = newline_of(text);
    let mut result = String::new();
    let lines = Lines {
        unit: DEFAULT_UNIT,
        newline,
        base: "",
    };
    write_value(&build(path, 0, value)?, Style::Lines(lines), &mut result);
    result.push_str(newline);
    Ok(result)
}

/// The value to write where nothing is yet for the keys of `path` from `from` on: objects and arrays around the value
/// that hold it at the rest of the path.
fn build(path: &[Key], from: usize, value: &Value) -> Result<Value, Error> {
    let mut built = value.clone();
    for depth in (from..path.len()).rev() {
        built = match path.get(depth) {
            Some(Key::Member(name)) => Value::Object(vec![Member {
                name: name.clone(),
                value: built,
            }]),
            Some(Key::Index(0)) => Value::Array(vec![built]),
            Some(Key::Index(index)) => {
                let path = path.get(..depth).unwrap_or_default().to_vec();
                return Err(Error::IndexOutOfRange {
                    path,
                    index: *index,
                    length: 0,
                });
            }
            None => built,
        };
    }
    Ok(built)
}

/// Where a path leads in a document.
enum Location<'a> {
    /// To the whole document.
    Root,
    /// To the entry at `index` of a container.
    Existing {
        container: &'a Node,
        entries: &'a [Entry],
        index: usize,
        entry: &'a Entry,
    },
    /// To nothing yet: the key at `depth` of the path is not in the container.
    Missing {
        container: &'a Node,
        entries: &'a [Entry],
        depth: usize,
    },
}

/// Where a path leads in a document, or why it cannot be followed there.
fn locate<'a>(root: &'a Node, path: &[Key]) -> Result<Location<'a>, Error> {
    let mut node = root;
    for (depth, key) in path.iter().enumerate() {
        let (entries, found) = match (&node.kind, key) {
            (NodeKind::Object(entries), Key::Member(name)) => (
                entries,
                entries
                    .iter()
                    .rposition(|entry| entry.name.as_deref() == Some(name.as_str())),
            ),
            (NodeKind::Array(entries), Key::Index(index)) => (entries, Some(*index)),
            (kind, key) => {
                let found = match kind {
                    NodeKind::Scalar(value) => value.kind(),
                    NodeKind::Array(_) => ValueKind::Array,
                    NodeKind::Object(_) => ValueKind::Object,
                };
                let path = path.get(..depth).unwrap_or_default().to_vec();
                return Err(Error::NotAContainer {
                    path,
                    found,
                    key: key.clone(),
                });
            }
        };
        let Some((index, entry)) = found.and_then(|index| Some((index, entries.get(index)?))) else {
            return Ok(Location::Missing {
                container: node,
                entries,
                depth,
            });
        };
        if depth + 1 == path.len() {
            return Ok(Location::Existing {
                container: node,
                entries,
                index,
                entry,
            });
        }
        node = &entry.value;
    }
    Ok(Location::Root)
}

/// A text being edited, and the habits a new line written into it follows.
struct File<'a> {
    text: &'a str,
    conventions: Conventions,
    /// Where the value of the whole document starts.
    root_start: usize,
}

/// An entry to add where a path leads to nothing yet.
struct NewEntry<'a> {
    /// The whole path.
    path: &'a [Key],
    /// The index in the path of the key of the new entry, the first that is missing.
    depth: usize,
    /// The value to set at the end of the path.
    value: &'a Value,
}

impl File<'_> {
    /// The splice that writes a new value over a value of a container, in the style of the container.
    fn replace(&self, container: &Node, old: &Node, value: &Value) -> Splice {
        let base = line_indent(self.text, old.start);
        let mut replacement = String::new();
        write_value(
            value,
            self.style(shape_of(self.text, container.start..container.end), base),
            &mut replacement,
        );
        Splice {
            start: old.start,
            end: old.end,
            replacement,
        }
    }

    fn style<'b>(&'b self, shape: Shape, base: &'b str) -> Style<'b> {
        match shape {
            Shape::OneLine => Style::Inline,
            Shape::SeveralLines => Style::Lines(Lines {
                unit: &self.conventions.unit,
                newline: self.conventions.newline,
                base,
            }),
        }
    }

    /// The text of a new entry in a style, with what the rest of the path needs around the value.
    fn entry_text(new: &NewEntry<'_>, style: Style<'_>) -> Result<String, Error> {
        let built = build(new.path, new.depth + 1, new.value)?;
        let mut text = String::new();
        match new.path.get(new.depth) {
            Some(Key::Member(name)) => write_member(name, &built, style, &mut text),
            Some(Key::Index(_)) | None => write_value(&built, style, &mut text),
        }
        Ok(text)
    }

    /// The splices that add a new entry at the end of a container.
    fn insert(&self, container: &Node, entries: &[Entry], new: &NewEntry<'_>) -> Result<Vec<Splice>, Error> {
        if let Some(Key::Index(index)) = new.path.get(new.depth)
            && *index > entries.len()
        {
            let path = new.path.get(..new.depth).unwrap_or_default().to_vec();
            return Err(Error::IndexOutOfRange {
                path,
                index: *index,
                length: entries.len(),
            });
        }
        match entries.last() {
            None => self.insert_into_empty(container, new),
            Some(last) => self.insert_after(container, last, new),
        }
    }

    /// Adds the first entry of an empty container: on a line of its own, unless the whole document is on one line.
    fn insert_into_empty(&self, container: &Node, new: &NewEntry<'_>) -> Result<Vec<Splice>, Error> {
        let text = self.text;
        let open = container.start;
        let close = container.end.saturating_sub(1);
        if self.conventions.shape == Shape::OneLine && container.start != self.root_start {
            let entry = File::entry_text(new, Style::Inline)?;
            return Ok(vec![Splice {
                start: open + 1,
                end: close,
                replacement: entry,
            }]);
        }
        let outer = line_indent(text, open);
        let inner = format!("{outer}{}", self.conventions.unit);
        let entry = File::entry_text(new, self.style(Shape::SeveralLines, &inner))?;
        let newline = self.conventions.newline;
        if is_at_line_start(text, close) && line_start(text, close) > open {
            let start = line_start(text, close);
            let replacement = format!("{inner}{entry}{newline}");
            return Ok(vec![Splice {
                start,
                end: start,
                replacement,
            }]);
        }
        let start = skip_spaces_back(text, open + 1..close);
        Ok(vec![Splice {
            start,
            end: close,
            replacement: format!("{newline}{inner}{entry}{newline}{outer}"),
        }])
    }

    /// Adds an entry after the last one of a container, with a comma between them, and with a comma after it too when
    /// the last one had one.
    fn insert_after(&self, container: &Node, last: &Entry, new: &NewEntry<'_>) -> Result<Vec<Splice>, Error> {
        let text = self.text;
        let anchor = last.comma.map_or(last.value.end, |comma| comma + 1);
        let trailing = if last.comma.is_some() { "," } else { "" };
        let mut splices = Vec::new();
        if last.comma.is_none() {
            splices.push(Splice {
                start: last.value.end,
                end: last.value.end,
                replacement: ",".to_string(),
            });
        }
        if shape_of(text, container.start..container.end) == Shape::OneLine {
            let entry = File::entry_text(new, Style::Inline)?;
            splices.push(Splice {
                start: anchor,
                end: anchor,
                replacement: format!(" {entry}{trailing}"),
            });
            return Ok(splices);
        }
        let indent = if is_at_line_start(text, last.start) {
            line_indent(text, last.start).to_string()
        } else {
            format!("{}{}", line_indent(text, container.start), self.conventions.unit)
        };
        let entry = File::entry_text(new, self.style(Shape::SeveralLines, &indent))?;
        let after = same_line_trivia_end(text, anchor);
        let point = if newline_length_at(text, after) > 0 {
            after
        } else {
            skip_spaces_back(text, anchor..after)
        };
        let replacement = format!("{}{indent}{entry}{trailing}", self.conventions.newline);
        splices.push(Splice {
            start: point,
            end: point,
            replacement,
        });
        Ok(splices)
    }
}

/// The splices that remove the entry at `index` of a container, with its line when it is alone on it, and fix the
/// commas: when the last entry goes and it had no comma after it, the one before it loses its comma too.
fn removal(text: &str, container: &Node, entries: &[Entry], (index, entry): (usize, &Entry)) -> Vec<Splice> {
    let content_end = entry.comma.map_or(entry.value.end, |comma| comma + 1);
    let after = same_line_trivia_end(text, content_end);
    let newline = newline_length_at(text, after);
    let is_whole_line = is_at_line_start(text, entry.start) && newline > 0;
    let open = container.start + 1;
    let close = container.end.saturating_sub(1);
    let (mut start, end) = if is_whole_line {
        (line_start(text, entry.start), after + newline)
    } else if skip_spaces(text, content_end) == close {
        // The last entry takes the spaces before it, so that the closing byte stays where the entries end.
        (skip_spaces_back(text, open..entry.start), content_end)
    } else {
        (entry.start, skip_spaces(text, content_end))
    };
    if entries.len() == 1 && is_white_space(text, open..start) && is_white_space(text, end..close) {
        // The only entry goes, and nothing else is inside: the container closes up, as `{}` or `[]`.
        return vec![Splice {
            start: open,
            end: close,
            replacement: String::new(),
        }];
    }
    let previous_comma = index
        .checked_sub(1)
        .and_then(|previous| entries.get(previous))
        .and_then(|previous| previous.comma);
    let mut splices = Vec::new();
    match (entry.comma, previous_comma) {
        (None, Some(comma)) if is_whole_line => splices.push(Splice {
            start: comma,
            end: comma + 1,
            replacement: String::new(),
        }),
        (None, Some(comma)) => start = comma,
        _ => {}
    }
    splices.push(Splice {
        start,
        end,
        replacement: String::new(),
    });
    splices
}

#[cfg(test)]
mod tests;
