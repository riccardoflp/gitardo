//! The JSON of what a feature asks and of what the extension answers: [`QuestionJson`], [`ValidationJson`] and
//! [`AnswerJson`]. What it runs on is in `target.rs`.

use std::path::PathBuf;

use model::contract::{Answer, Choice, FileQuestion, Question, QuestionKind, Text, Validation};
use model::worktree::{FolderBase, WorktreeNameRule};
use serde::{Deserialize, Serialize};

/// An answer a person gives, or a yes or no an operation offered is given before it runs. The other kinds of `Answer`
/// are the engine's own readings and never cross.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AnswerJson {
    /// Text typed.
    Text {
        /// The text.
        text: String,
    },
    /// A choice picked.
    Choice {
        /// Its identifier.
        identifier: String,
    },
    /// Choices picked together.
    Choices {
        /// Their identifiers.
        identifiers: Vec<String>,
    },
    /// A question confirmed.
    Confirmed,
    /// A file or a folder chosen.
    Path {
        /// Where it is.
        path: String,
    },
    /// Yes or no, as an operation offered is told what the engine found before it was offered: **Delete anyway** is
    /// told the branch is not merged.
    Flag {
        /// Yes or no.
        value: bool,
    },
}

impl From<AnswerJson> for Answer {
    fn from(answer: AnswerJson) -> Answer {
        match answer {
            AnswerJson::Text { text } => Answer::Text(text),
            AnswerJson::Choice { identifier } => Answer::Choice(identifier),
            AnswerJson::Choices { identifiers } => Answer::Choices(identifiers),
            AnswerJson::Confirmed => Answer::Confirmed,
            AnswerJson::Path { path } => Answer::Path(PathBuf::from(path)),
            AnswerJson::Flag { value } => Answer::Flag(value),
        }
    }
}

impl AnswerJson {
    /// The JSON of an answer a person could give; none for the engine's own readings.
    #[must_use]
    pub fn of(answer: &Answer) -> Option<AnswerJson> {
        Some(match answer {
            Answer::Text(text) => AnswerJson::Text { text: text.clone() },
            Answer::Choice(identifier) => AnswerJson::Choice {
                identifier: identifier.clone(),
            },
            Answer::Choices(identifiers) => AnswerJson::Choices {
                identifiers: identifiers.clone(),
            },
            Answer::Confirmed => AnswerJson::Confirmed,
            Answer::Path(path) => AnswerJson::Path {
                path: path.to_string_lossy().into_owned(),
            },
            Answer::Flag(value) => AnswerJson::Flag { value: *value },
            _ => return None,
        })
    }
}

/// A question of a feature.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionJson {
    /// The key of its answer.
    pub key: String,
    /// How it is answered.
    pub kind: QuestionKindJson,
    /// Its title.
    pub title: String,
    /// What is said under it.
    pub detail: Option<String>,
    /// The answer it starts with.
    pub default: Option<AnswerJson>,
}

/// How a question is answered.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum QuestionKindJson {
    /// Text typed in a box.
    Text {
        /// What the empty box says.
        placeholder: Option<String>,
        /// What the text must be.
        validation: ValidationJson,
    },
    /// Text typed and hidden.
    Secret {
        /// What the text must be.
        validation: ValidationJson,
    },
    /// One of a few choices.
    Pick {
        /// The choices.
        choices: Vec<ChoiceJson>,
    },
    /// One of many choices, searched by typing.
    Search {
        /// The choices.
        choices: Vec<ChoiceJson>,
    },
    /// Some of the choices.
    PickMany {
        /// The choices.
        choices: Vec<ChoiceJson>,
    },
    /// A question to confirm.
    Confirm {
        /// The words of the button that confirms.
        accept: String,
    },
    /// A file to open.
    OpenFile(FileQuestionJson),
    /// A file to save.
    SaveFile(FileQuestionJson),
}

/// One choice of a pick.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceJson {
    /// What the answer names it by.
    pub identifier: String,
    /// What it says.
    pub label: String,
    /// What is said beside it.
    pub description: Option<String>,
}

/// Where a file is chosen.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileQuestionJson {
    /// The folder it starts in.
    pub folder: Option<String>,
    /// The name it proposes.
    pub file_name: Option<String>,
    /// The kinds of files shown.
    pub filters: Vec<FileFilterJson>,
}

/// A kind of file shown.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileFilterJson {
    /// Its name.
    pub name: String,
    /// The extensions of its files.
    pub extensions: Vec<String>,
}

/// What typed text must be (`Validation` of `model`), sent back by the extension in `question/validate`. Every kind is
/// an object with a `kind`, those without data too, so that one form is read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ValidationJson {
    /// Anything, even nothing.
    Anything,
    /// A name git takes, none of those taken.
    ReferenceName {
        /// The names taken.
        taken: Vec<String>,
        /// What is said of a name taken.
        taken_message: String,
    },
    /// The URL of a remote repository.
    Url,
    /// A new message for a commit, not the old one.
    NewMessage {
        /// The message it has.
        old: String,
    },
    /// Anything but nothing.
    Required {
        /// What is said while it is empty.
        message: String,
    },
    /// Nothing, or a name git takes for a branch.
    OptionalReferenceName,
    /// A folder inside the repository where no submodule is.
    SubmoduleFolder {
        /// The folders of the submodules.
        submodules: Vec<String>,
    },
    /// The name of a worktree and of its folder.
    WorktreeName {
        /// The branches there are, when the name is a new branch's too.
        branches: Option<Vec<String>>,
        /// The name taken when nothing is typed.
        empty_name: Option<String>,
        /// The folder of the worktrees, where the folder goes by the name's path.
        worktrees: Option<String>,
        /// The folder the new one goes in beside a worktree that is elsewhere.
        beside: Option<String>,
        /// The folders of the worktrees there are.
        taken_folders: Vec<String>,
    },
    /// Anything but nothing that matches a regular expression.
    Pattern {
        /// The expression.
        pattern: String,
    },
}

fn text_of(text: &Text) -> String {
    text.as_str().to_owned()
}

fn path_of(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

impl From<&Validation> for ValidationJson {
    fn from(validation: &Validation) -> ValidationJson {
        match validation {
            Validation::Anything => ValidationJson::Anything,
            Validation::ReferenceName { taken, taken_message } => ValidationJson::ReferenceName {
                taken: taken.clone(),
                taken_message: text_of(taken_message),
            },
            Validation::Url => ValidationJson::Url,
            Validation::NewMessage { old } => ValidationJson::NewMessage { old: old.clone() },
            Validation::Required(message) => ValidationJson::Required {
                message: text_of(message),
            },
            Validation::OptionalReferenceName => ValidationJson::OptionalReferenceName,
            Validation::SubmoduleFolder { submodules } => ValidationJson::SubmoduleFolder {
                submodules: submodules.clone(),
            },
            Validation::WorktreeName(rule) => {
                let (worktrees, beside) = match &rule.base {
                    FolderBase::Worktrees(folder) => (Some(folder.clone()), None),
                    FolderBase::Beside(folder) => (None, Some(path_of(folder))),
                };
                ValidationJson::WorktreeName {
                    branches: rule.branches.clone(),
                    empty_name: rule.empty_name.clone(),
                    worktrees,
                    beside,
                    taken_folders: rule.taken_folders.iter().map(|folder| path_of(folder)).collect(),
                }
            }
            Validation::Pattern(pattern) => ValidationJson::Pattern {
                pattern: pattern.clone(),
            },
        }
    }
}

impl From<ValidationJson> for Validation {
    fn from(validation: ValidationJson) -> Validation {
        match validation {
            ValidationJson::Anything => Validation::Anything,
            ValidationJson::ReferenceName { taken, taken_message } => Validation::ReferenceName {
                taken,
                taken_message: Text::from(taken_message),
            },
            ValidationJson::Url => Validation::Url,
            ValidationJson::NewMessage { old } => Validation::NewMessage { old },
            ValidationJson::Required { message } => Validation::Required(Text::from(message)),
            ValidationJson::OptionalReferenceName => Validation::OptionalReferenceName,
            ValidationJson::SubmoduleFolder { submodules } => Validation::SubmoduleFolder { submodules },
            ValidationJson::WorktreeName {
                branches,
                empty_name,
                worktrees,
                beside,
                taken_folders,
            } => Validation::WorktreeName(Box::new(WorktreeNameRule {
                branches,
                empty_name,
                base: match (worktrees, beside) {
                    (_, Some(beside)) => FolderBase::Beside(PathBuf::from(beside)),
                    (worktrees, None) => FolderBase::Worktrees(worktrees.unwrap_or_default()),
                },
                taken_folders: taken_folders.into_iter().map(PathBuf::from).collect(),
            })),
            ValidationJson::Pattern { pattern } => Validation::Pattern(pattern),
        }
    }
}

fn choices_of(choices: &[Choice]) -> Vec<ChoiceJson> {
    choices
        .iter()
        .map(|choice| ChoiceJson {
            identifier: choice.identifier.clone(),
            label: text_of(&choice.label),
            description: choice.description.as_ref().map(text_of),
        })
        .collect()
}

fn file_question_of(question: &FileQuestion) -> FileQuestionJson {
    FileQuestionJson {
        folder: Some(path_of(&question.folder)).filter(|folder| !folder.is_empty()),
        file_name: question.file_name.clone(),
        filters: question
            .filters
            .iter()
            .map(|filter| FileFilterJson {
                name: text_of(&filter.name),
                extensions: filter.extensions.clone(),
            })
            .collect(),
    }
}

impl From<&Question> for QuestionJson {
    fn from(question: &Question) -> QuestionJson {
        let kind = match &question.kind {
            QuestionKind::Text {
                placeholder,
                validation,
            } => QuestionKindJson::Text {
                placeholder: placeholder.as_ref().map(text_of),
                validation: validation.into(),
            },
            QuestionKind::Secret { validation } => QuestionKindJson::Secret {
                validation: validation.into(),
            },
            QuestionKind::Pick { choices } => QuestionKindJson::Pick {
                choices: choices_of(choices),
            },
            QuestionKind::Search { choices } => QuestionKindJson::Search {
                choices: choices_of(choices),
            },
            QuestionKind::PickMany { choices } => QuestionKindJson::PickMany {
                choices: choices_of(choices),
            },
            QuestionKind::Confirm { accept } => QuestionKindJson::Confirm {
                accept: text_of(accept),
            },
            QuestionKind::OpenFile(file) => QuestionKindJson::OpenFile(file_question_of(file)),
            QuestionKind::SaveFile(file) => QuestionKindJson::SaveFile(file_question_of(file)),
        };
        QuestionJson {
            key: question.key.0.to_owned(),
            kind,
            title: text_of(&question.title),
            detail: question.detail.as_ref().map(text_of),
            default: question.default.as_ref().and_then(AnswerJson::of),
        }
    }
}
