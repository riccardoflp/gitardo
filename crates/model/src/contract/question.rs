//! A question a feature asks the user, shown by the application in its own dialogs, pickers and input boxes.

use super::{Answer, AnswerKey, Text};
use crate::branch::validate_name;

/// A question, and where its answer goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    /// Where the answer comes back.
    pub key: AnswerKey,
    /// What kind of answer it wants.
    pub kind: QuestionKind,
    /// Its title: **Create branch**.
    pub title: Text,
    /// What will happen with the answer, when the title does not say it all.
    pub detail: Option<Text>,
    /// The answer it starts from.
    pub default: Option<Answer>,
}

/// What kind of answer a question wants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestionKind {
    /// Text typed in an input box, checked while it is typed.
    Text {
        /// What the empty box shows: `feature/my-change`.
        placeholder: Option<Text>,
        /// What the text must be.
        validation: Validation,
    },
    /// One choice among several.
    Pick {
        /// The choices, in order.
        choices: Vec<Choice>,
    },
    /// Any number of choices, ticked.
    PickMany {
        /// The choices, in order.
        choices: Vec<Choice>,
    },
    /// Yes or no; the button that accepts says what will happen.
    Confirm {
        /// The words of the button that accepts: **Stage all and commit**.
        accept: Text,
    },
}

/// One choice of a pick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    /// What comes back in the answer when it is picked.
    pub identifier: String,
    /// What the user reads: **Create and check out**.
    pub label: Text,
    /// The words beside it: **Stay on the current branch**.
    pub description: Option<Text>,
}

/// What typed text must be for a question to accept it. A pure rule, so that the box can say what is wrong while the
/// text is typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Validation {
    /// Anything, even nothing.
    Anything,
    /// A name git takes for a branch, a tag or a remote (`branch.validate-name`), and none of the names taken.
    ReferenceName {
        /// The names already taken.
        taken: Vec<String>,
        /// What is said when the name is taken: "A branch with that name already exists".
        taken_message: Text,
    },
    /// The URL of a remote repository: not empty, and not starting with `-`, which git would take for an option.
    Url,
    /// A new message for a commit: not empty, and not the message it has, spaces and blank lines around either left
    /// out.
    NewMessage {
        /// The message the commit has.
        old: String,
    },
    /// Anything but nothing, spaces left out: what is said while it is empty.
    Required(Text),
}

impl Validation {
    /// What is wrong with a text, or nothing when the question can accept it.
    #[must_use]
    pub fn problem(&self, text: &str) -> Option<Text> {
        match self {
            Validation::Anything => None,
            Validation::ReferenceName { taken, taken_message } => {
                validate_name(text).or_else(|| taken.iter().any(|name| name == text).then(|| taken_message.clone()))
            }
            Validation::Url => url_problem(text),
            Validation::NewMessage { old } => {
                if text.trim().is_empty() {
                    Some(Text::from("Enter a message."))
                } else if text.trim() == old.trim() {
                    Some(Text::from("The message is unchanged."))
                } else {
                    None
                }
            }
            Validation::Required(message) => text.trim().is_empty().then(|| message.clone()),
        }
    }
}

/// What is wrong with the URL of a remote, spaces around it left out: "Enter a URL" when there is none, "That is not a
/// URL" when it starts with `-`.
fn url_problem(text: &str) -> Option<Text> {
    let url = text.trim();
    if url.is_empty() {
        Some(Text::from("Enter a URL"))
    } else if url.starts_with('-') {
        Some(Text::from("That is not a URL"))
    } else {
        None
    }
}
