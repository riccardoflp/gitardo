//! The todo list git reads for an interactive rebase, and the files of the new messages of the commits reworded.

use std::fmt::Write as _;

use crate::contract::Text;

/// What a step of an interactive rebase does with its commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TodoAction {
    /// Keep it.
    Pick,
    /// Keep it with a new message.
    Reword,
    /// Stop on it to change it.
    Edit,
    /// Join it to the one before, the two messages kept.
    Squash,
    /// Join it to the one before, its message left out.
    Fixup,
    /// Leave it out.
    Drop,
}

impl TodoAction {
    /// The word of the todo list.
    fn word(self) -> &'static str {
        match self {
            TodoAction::Pick | TodoAction::Reword => "pick",
            TodoAction::Edit => "edit",
            TodoAction::Squash => "squash",
            TodoAction::Fixup => "fixup",
            TodoAction::Drop => "drop",
        }
    }
}

/// The new message of a commit reworded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reword<'a> {
    /// The new message, or only its new subject.
    pub message: &'a str,
    /// Whether it is only a new subject, the body of the original message kept.
    pub is_subject_only: bool,
    /// The original message.
    pub original: &'a str,
}

/// A step of the rebase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TodoStep<'a> {
    /// What it does.
    pub action: TodoAction,
    /// The commit, by its full hash.
    pub commit: &'a str,
    /// The new message, for a reword.
    pub reword: Option<Reword<'a>>,
}

/// The todo list, and the files of the messages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TodoList {
    /// The text git reads.
    pub text: String,
    /// Each file of a message: its name in the folder, and what it holds.
    pub files: Vec<(String, String)>,
}

/// The todo list of steps oldest first, its message files in a folder; or why it cannot be.
///
/// Each step is a line `<action> <hash>`, but a reword, which is `pick <hash>` then `exec git commit --amend -F
/// "<folder>/message-<n>.txt"`, `<n>` being the step's index. A message given as a subject only keeps the body of the
/// original message after it.
///
/// Feature: rebase.build-todo-list
///
/// # Errors
///
/// When every step drops its commit, when the first commit kept is to be squashed into nothing, or when a hash is
/// empty or starts with `-`.
pub fn build_todo_list(steps: &[TodoStep<'_>], folder: &str) -> Result<TodoList, Text> {
    if steps
        .iter()
        .any(|step| step.commit.is_empty() || step.commit.starts_with('-'))
    {
        return Err(Text::from("A commit of the list has no hash git can read."));
    }
    match steps.iter().find(|step| step.action != TodoAction::Drop) {
        None => {
            return Err(Text::from(
                "Every commit is dropped. To remove them all, reset the branch instead.",
            ));
        }
        Some(first) if matches!(first.action, TodoAction::Squash | TodoAction::Fixup) => {
            return Err(Text::from(
                "The oldest commit that is kept has nothing before it to be squashed into.",
            ));
        }
        Some(_) => {}
    }
    let folder = folder.replace('\\', "/");
    let mut list = TodoList {
        text: String::new(),
        files: Vec::new(),
    };
    for (index, step) in steps.iter().enumerate() {
        let _ = writeln!(list.text, "{} {}", step.action.word(), step.commit);
        if let (TodoAction::Reword, Some(reword)) = (step.action, step.reword) {
            let name = format!("message-{index}.txt");
            let _ = writeln!(list.text, "exec git commit --amend -F \"{folder}/{name}\"");
            list.files.push((name, reworded(reword)));
        }
    }
    Ok(list)
}

/// What the file of a new message holds.
fn reworded(reword: Reword<'_>) -> String {
    let mut message = reword.message.trim().to_owned();
    if reword.is_subject_only {
        let body = body_of(reword.original);
        if !body.is_empty() {
            message.push_str("\n\n");
            message.push_str(body);
        }
    }
    message.push('\n');
    message
}

/// What follows the first blank line of a message, trimmed.
#[must_use]
pub(crate) fn body_of(message: &str) -> &str {
    let mut offset = 0;
    let mut seen_text = false;
    for line in message.split_inclusive('\n') {
        offset += line.len();
        if line.trim().is_empty() {
            if seen_text {
                return message.get(offset..).unwrap_or_default().trim();
            }
        } else {
            seen_text = true;
        }
    }
    ""
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{Reword, TodoAction, TodoList, TodoStep, build_todo_list};

    fn step(action: TodoAction, commit: &str) -> TodoStep<'_> {
        TodoStep {
            action,
            commit,
            reword: None,
        }
    }

    #[test]
    fn rebase_build_todo_list__a_reword_of_its_subject_keeps_the_body() {
        let steps = [
            step(TodoAction::Pick, "A"),
            TodoStep {
                reword: Some(Reword {
                    message: "Better words",
                    is_subject_only: true,
                    original: "Old words\n\nBody.\n",
                }),
                ..step(TodoAction::Reword, "B")
            },
            step(TodoAction::Fixup, "C"),
            step(TodoAction::Drop, "D"),
        ];
        assert_eq!(
            build_todo_list(&steps, "C:\\repository\\.git\\gitardo"),
            Ok(TodoList {
                text: String::from(
                    "pick A\npick B\nexec git commit --amend -F \"C:/repository/.git/gitardo/message-1.txt\"\nfixup C\ndrop D\n"
                ),
                files: vec![(String::from("message-1.txt"), String::from("Better words\n\nBody.\n"))],
            })
        );
    }

    #[test]
    fn rebase_build_todo_list__a_message_given_whole_is_written_alone() {
        let steps = [TodoStep {
            reword: Some(Reword {
                message: "  New subject\n\nNew body.  ",
                is_subject_only: false,
                original: "Old words\n\nBody.",
            }),
            ..step(TodoAction::Reword, "B")
        }];
        let files = build_todo_list(&steps, "folder").map(|list| list.files);
        assert_eq!(
            files,
            Ok(vec![(
                String::from("message-0.txt"),
                String::from("New subject\n\nNew body.\n")
            )])
        );
    }

    #[test]
    fn rebase_build_todo_list__all_dropped_or_a_squash_first_is_refused() {
        let dropped = build_todo_list(&[step(TodoAction::Drop, "A"), step(TodoAction::Drop, "B")], "folder");
        assert_eq!(
            dropped.map_err(|text| text.as_str().to_owned()),
            Err(String::from(
                "Every commit is dropped. To remove them all, reset the branch instead."
            ))
        );
        let squashed = build_todo_list(&[step(TodoAction::Drop, "A"), step(TodoAction::Squash, "B")], "folder");
        assert_eq!(
            squashed.map_err(|text| text.as_str().to_owned()),
            Err(String::from(
                "The oldest commit that is kept has nothing before it to be squashed into."
            ))
        );
        assert!(build_todo_list(&[step(TodoAction::Pick, "-x")], "folder").is_err());
        assert!(build_todo_list(&[step(TodoAction::Pick, "")], "folder").is_err());
    }
}
