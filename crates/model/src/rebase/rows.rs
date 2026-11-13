//! The rows of the interactive rebase page as the page hands them to `rebase.interactive`: one line each, oldest first,
//! `<action> <full hash>`, and for a reword the new subject after the hash.

use std::fmt::Write as _;

use super::TodoAction;
use crate::CommitId;

/// A row of the interactive rebase page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TodoRow {
    /// What the rebase does with the commit.
    pub action: TodoAction,
    /// The commit.
    pub commit: CommitId,
    /// For a reword, the new subject; the body of the message stays.
    pub new_subject: Option<String>,
}

/// The word of an action in the rows.
fn word(action: TodoAction) -> &'static str {
    match action {
        TodoAction::Pick => "pick",
        TodoAction::Reword => "reword",
        TodoAction::Edit => "edit",
        TodoAction::Squash => "squash",
        TodoAction::Fixup => "fixup",
        TodoAction::Drop => "drop",
    }
}

/// The rows, oldest first, as `rebase.interactive` reads them.
#[must_use]
pub fn write_todo_rows(rows: &[TodoRow]) -> String {
    let mut text = String::new();
    for row in rows {
        let _ = write!(text, "{} {}", word(row.action), row.commit);
        if let (TodoAction::Reword, Some(subject)) = (row.action, &row.new_subject) {
            let _ = write!(text, " {}", subject.replace(['\n', '\r'], " "));
        }
        text.push('\n');
    }
    text
}

/// The rows of a text [`write_todo_rows`] wrote; none when a line is not a row.
#[must_use]
pub fn read_todo_rows(text: &str) -> Option<Vec<TodoRow>> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut parts = line.trim_start().splitn(3, ' ');
            let action = match parts.next()? {
                "pick" => TodoAction::Pick,
                "reword" => TodoAction::Reword,
                "edit" => TodoAction::Edit,
                "squash" => TodoAction::Squash,
                "fixup" => TodoAction::Fixup,
                "drop" => TodoAction::Drop,
                _ => return None,
            };
            let commit = CommitId::parse(parts.next()?)?;
            let new_subject =
                (action == TodoAction::Reword).then(|| parts.next().unwrap_or_default().trim().to_owned());
            Some(TodoRow {
                action,
                commit,
                new_subject,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{TodoAction, TodoRow, read_todo_rows, write_todo_rows};
    use crate::CommitId;

    #[test]
    fn reads_the_rows_it_wrote() {
        let rows = vec![
            TodoRow {
                action: TodoAction::Pick,
                commit: CommitId::parse(&"a".repeat(40)).unwrap(),
                new_subject: None,
            },
            TodoRow {
                action: TodoAction::Reword,
                commit: CommitId::parse(&"b".repeat(40)).unwrap(),
                new_subject: Some(String::from("Better words")),
            },
        ];
        let text = write_todo_rows(&rows);
        assert_eq!(read_todo_rows(&text), Some(rows));
        assert_eq!(read_todo_rows("frob aaaa"), None);
    }
}
