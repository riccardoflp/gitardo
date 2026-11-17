//! Where a value is written in the text of a file of JSON with comments, so that an editor can open the file there.

use crate::error::{Key, Position};
use crate::parse::{Dialect, NodeKind, parse_document};

/// The line and the column where the value a path of keys leads to is written in a text of JSON with comments: the
/// quote of the name of a member, the first byte of an item of an array, and the start of the value of the whole
/// document for an empty path. Nothing when the text is not valid JSON with comments, or the path leads nowhere in it.
#[must_use]
pub fn position_of(text: &str, path: &[Key]) -> Option<Position> {
    let root = parse_document(text, Dialect::WithComments).ok()?;
    let mut node = &root;
    let mut start = root.start;
    for key in path {
        let entry = match (&node.kind, key) {
            // When a name is written more than once, the last one counts, as it does when the text is read.
            (NodeKind::Object(entries), Key::Member(name)) => entries
                .iter()
                .rfind(|entry| entry.name.as_deref() == Some(name.as_str())),
            (NodeKind::Array(entries), Key::Index(index)) => entries.get(*index),
            (NodeKind::Scalar(_) | NodeKind::Array(_) | NodeKind::Object(_), _) => None,
        }?;
        start = entry.start;
        node = &entry.value;
    }
    Some(Position::of(text, start))
}

#[cfg(test)]
mod tests {
    use super::position_of;
    use crate::error::{Key, Position};

    const TASKS: &str = r#"{
  // The tasks of the repository.
  "version": "2.0.0",
  "tasks": [
    {
      "type": "shell",
      "label": "build"
    },
    { "label": "test" }
  ]
}
"#;

    fn label(index: usize) -> Vec<Key> {
        vec![
            Key::Member(String::from("tasks")),
            Key::Index(index),
            Key::Member(String::from("label")),
        ]
    }

    #[test]
    fn says_the_line_and_the_column_of_the_name_of_a_member() {
        assert_eq!(position_of(TASKS, &label(0)), Some(Position { line: 7, column: 7 }));
        assert_eq!(position_of(TASKS, &label(1)), Some(Position { line: 9, column: 7 }));
    }

    #[test]
    fn says_where_an_item_of_an_array_and_the_whole_document_start() {
        let item = [Key::Member(String::from("tasks")), Key::Index(1)];
        assert_eq!(position_of(TASKS, &item), Some(Position { line: 9, column: 5 }));
        assert_eq!(position_of(TASKS, &[]), Some(Position { line: 1, column: 1 }));
    }

    #[test]
    fn says_nothing_for_a_path_that_leads_nowhere_or_a_text_that_is_not_json() {
        assert_eq!(position_of(TASKS, &label(2)), None);
        assert_eq!(position_of(TASKS, &[Key::Member(String::from("inputs"))]), None);
        let through_a_string = [Key::Member(String::from("version")), Key::Index(0)];
        assert_eq!(position_of(TASKS, &through_a_string), None);
        assert_eq!(position_of("{ \"tasks\": [", &label(0)), None);
    }
}
