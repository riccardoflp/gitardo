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
    /// A secret typed in an input box that hides what is typed: a token.
    Secret {
        /// What the secret must be.
        validation: Validation,
    },
    /// One choice among several.
    Pick {
        /// The choices, in order.
        choices: Vec<Choice>,
    },
    /// One choice among many, found by typing: a box over the choices keeps those that [`Choice::matches`] what is
    /// typed.
    Search {
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
    /// A file to open, chosen in a dialog; the answer is its path.
    OpenFile(FileQuestion),
    /// Where to save a file, chosen in a dialog; the answer is its path.
    SaveFile(FileQuestion),
}

/// Where a dialog that chooses a file starts, and which files it shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileQuestion {
    /// The folder it opens in.
    pub folder: std::path::PathBuf,
    /// The name it proposes, when saving.
    pub file_name: Option<String>,
    /// The kinds of files it offers to show, the first one shown at first.
    pub filters: Vec<FileFilter>,
}

/// A kind of files a dialog offers to show: **Patches**, `.patch` and `.diff`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFilter {
    /// Its name: **Patches**.
    pub name: Text,
    /// The extensions of its files, without the dot; none for every file.
    pub extensions: Vec<String>,
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

impl Choice {
    /// Whether what is typed in the box of a search finds the choice: its label, its description or its identifier
    /// holds it, whatever the case; nothing typed finds every choice.
    #[must_use]
    pub fn matches(&self, typed: &str) -> bool {
        let typed = typed.trim().to_lowercase();
        [
            Some(self.label.as_str()),
            self.description.as_ref().map(Text::as_str),
            Some(self.identifier.as_str()),
        ]
        .into_iter()
        .flatten()
        .any(|text| text.to_lowercase().contains(&typed))
    }
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
    /// Nothing, or a name git takes for a branch (`branch.validate-name`): the branch a submodule follows.
    OptionalReferenceName,
    /// A folder inside the repository, from its root, where no submodule is (`submodule.add`).
    SubmoduleFolder {
        /// The folders of the submodules, from the root of the repository.
        submodules: Vec<String>,
    },
    /// The name of a worktree and of its folder, with where the folder goes said while it is typed (`worktree.`).
    WorktreeName(Box<crate::worktree::WorktreeNameRule>),
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
            Validation::OptionalReferenceName => (!text.trim().is_empty()).then(|| validate_name(text.trim()))?,
            Validation::SubmoduleFolder { submodules } => crate::submodule::folder_problem(text, submodules),
            Validation::WorktreeName(rule) => rule.problem(text),
        }
    }

    /// Something to know about a text the question can accept, said under the box while it is typed: where the
    /// folder of a worktree goes.
    #[must_use]
    pub fn hint(&self, text: &str) -> Option<Text> {
        match self {
            Validation::WorktreeName(rule) => rule.hint(text),
            Validation::Anything
            | Validation::ReferenceName { .. }
            | Validation::Url
            | Validation::NewMessage { .. }
            | Validation::Required(_)
            | Validation::OptionalReferenceName
            | Validation::SubmoduleFolder { .. } => None,
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

#[cfg(test)]
mod tests {
    use super::{Choice, Text};

    #[test]
    fn a_choice_is_found_by_its_label_its_description_or_its_identifier_whatever_the_case() {
        let choice = Choice {
            identifier: String::from("/work/app.worktrees/feature/login"),
            label: Text::from("feature/login"),
            description: Some(Text::from("2 not committed")),
        };
        for typed in ["", "LOGIN", "committed", "app.worktrees", " feature "] {
            assert!(choice.matches(typed), "{typed}");
        }
        assert!(!choice.matches("typo"));
    }
}
