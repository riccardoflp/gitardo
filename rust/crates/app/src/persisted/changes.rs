//! What the Changes page keeps between runs (`changes.pane`): the message of the commit box of each repository, by its
//! folder, under `commitMessages`; a repository whose message is empty is not written.

use std::path::{Path, PathBuf};

use json::Value;

use super::{MOST_RECENT, member, same_folder};

/// The message of the commit box of a repository.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RepositoryMessage {
    /// The folder of the repository.
    pub folder: PathBuf,
    /// The message, as typed.
    pub message: String,
}

/// The message kept for a repository; empty when none is.
#[must_use]
pub fn message_of<'a>(list: &'a [RepositoryMessage], folder: &Path) -> &'a str {
    list.iter()
        .find(|kept| same_folder(&kept.folder, folder))
        .map_or("", |kept| kept.message.as_str())
}

/// Keeps the message of a repository, first in the list, and forgets it once it is empty; at most [`MOST_RECENT`]
/// repositories are kept.
pub fn keep_message(list: &mut Vec<RepositoryMessage>, folder: &Path, message: &str) {
    list.retain(|kept| !same_folder(&kept.folder, folder));
    if !message.is_empty() {
        list.insert(
            0,
            RepositoryMessage {
                folder: folder.to_path_buf(),
                message: message.to_owned(),
            },
        );
    }
    list.truncate(MOST_RECENT);
}

/// The message of a repository, as written.
pub(super) fn message_value(kept: &RepositoryMessage) -> Value {
    Value::Object(vec![
        member("folder", Value::String(kept.folder.to_string_lossy().into_owned())),
        member("message", Value::String(kept.message.clone())),
    ])
}

/// The message of a repository read back; none without a folder or a message.
pub(super) fn repository_message(value: &Value) -> Option<RepositoryMessage> {
    Some(RepositoryMessage {
        folder: PathBuf::from(value.get("folder")?.as_str()?),
        message: value.get("message")?.as_str()?.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{RepositoryMessage, keep_message, message_of};
    use crate::persisted::Persisted;

    #[test]
    fn a_message_kept_reads_back_and_an_empty_one_is_forgotten() {
        let mut persisted = Persisted::default();
        keep_message(
            &mut persisted.commit_messages,
            Path::new("/work"),
            "Fix the parser\n\nWith a body",
        );
        let read = Persisted::from_text(&persisted.to_text());
        assert_eq!(
            read.commit_messages,
            [RepositoryMessage {
                folder: PathBuf::from("/work"),
                message: String::from("Fix the parser\n\nWith a body"),
            }]
        );
        assert_eq!(
            message_of(&read.commit_messages, Path::new("/work")),
            "Fix the parser\n\nWith a body"
        );
        keep_message(&mut persisted.commit_messages, Path::new("/work"), "");
        assert_eq!(persisted.commit_messages, Vec::<RepositoryMessage>::new());
        assert_eq!(message_of(&persisted.commit_messages, Path::new("/work")), "");
    }
}
