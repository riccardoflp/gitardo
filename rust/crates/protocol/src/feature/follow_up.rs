//! The JSON of the follow-ups that are the extension's to carry out once a feature ran.

use std::collections::BTreeMap;
use std::path::Path;

use model::contract::{FollowUp, OfferedRun, Text};
use serde::{Deserialize, Serialize};

use super::question::AnswerJson;
use super::target::TargetJson;

/// A follow-up the extension carries out (`FollowUp` of `model`). The engine keeps those that are its own, and the ones
/// whose views have not moved yet come with their areas.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum FollowUpJson {
    /// A message.
    Say {
        /// Its words.
        text: String,
    },
    /// The graph selects `HEAD`.
    SelectHead,
    /// The graph selects a commit.
    SelectCommit {
        /// Its id.
        commit: String,
    },
    /// The graph selects the commit that many before `HEAD`.
    SelectBeforeHead {
        /// How many.
        count: usize,
    },
    /// The page of a commit opens.
    OpenCommitPage {
        /// Its id.
        commit: String,
    },
    /// The commit box is emptied.
    EmptyCommitMessage,
    /// The commit box takes the message git prepared.
    PreparedMessage,
    /// The commit box takes a message.
    FillCommitMessage {
        /// The message.
        message: String,
    },
    /// The pull requests are read again.
    ReadPullRequests,
    /// A feature is offered, with the words of its button.
    Offer {
        /// The feature.
        feature: String,
        /// The words of the button.
        label: String,
    },
    /// Text is copied to the clipboard.
    CopyToClipboard {
        /// The text.
        text: String,
    },
    /// An address opens in the browser.
    OpenUrl {
        /// The address.
        url: String,
    },
    /// The review of the conflicts opens on files.
    OpenReview {
        /// The files.
        paths: Vec<String>,
    },
    /// The merge page of a file closes.
    CloseMergePage {
        /// The file.
        path: String,
    },
    /// A folder opens in a new window.
    OpenWindow {
        /// The folder.
        folder: String,
    },
    /// The repository shown becomes the one in a folder.
    ShowFolder {
        /// The folder.
        folder: String,
    },
    /// The values given for the inputs of the format of branch names are kept, for the next time.
    RememberInputs {
        /// The values, by the identifier of each input.
        values: BTreeMap<String, String>,
    },
    /// A file opens in an editor at a line.
    EditFile {
        /// The file.
        path: String,
        /// The line, from 1.
        line: usize,
    },
    /// An operation is offered with a button, on a target of its own and with answers given: **Delete anyway** after
    /// git kept a branch it found not fully merged, **Remove anyway** after git refused to remove a worktree with
    /// changes, **Choose What It Gets…** once a worktree is made.
    OfferRun {
        /// The words of the button.
        label: String,
        /// The feature it runs.
        feature: String,
        /// What it runs on, boxed for it is the largest of the follow-ups by far.
        target: Box<TargetJson>,
        /// The answers it is given before it asks, by their keys.
        answers: BTreeMap<String, AnswerJson>,
    },
    /// A setting is written in the user's settings of VS Code, by the desktop's name (1.6, `ai.choose-model`).
    WriteSetting {
        /// The setting, as the desktop names it: `ai.model`.
        key: String,
        /// Its value; null removes it.
        value: Option<serde_json::Value>,
    },
}

fn text_of(text: &Text) -> String {
    text.as_str().to_owned()
}

fn path_of(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// The JSON of an operation offered; none when its target or one of its answers cannot cross, since it would then run
/// otherwise than offered.
fn offer_run(offered: &OfferedRun) -> Option<FollowUpJson> {
    let answers = offered
        .answers
        .iter()
        .map(|(key, answer)| Some((key.0.to_owned(), AnswerJson::of(answer)?)))
        .collect::<Option<BTreeMap<_, _>>>()?;
    Some(FollowUpJson::OfferRun {
        label: text_of(&offered.label),
        feature: offered.feature.0.to_owned(),
        target: Box::new(TargetJson::of(&offered.target)?),
        answers,
    })
}

impl FollowUpJson {
    /// The JSON of a follow-up that is the extension's to carry out; none for the engine's own and for those not
    /// moved yet.
    #[must_use]
    pub fn of(follow_up: &FollowUp) -> Option<FollowUpJson> {
        Some(match follow_up {
            FollowUp::Say(text) => FollowUpJson::Say { text: text_of(text) },
            FollowUp::SelectHead => FollowUpJson::SelectHead,
            FollowUp::SelectCommit(commit) => FollowUpJson::SelectCommit {
                commit: commit.as_str().to_owned(),
            },
            FollowUp::SelectBeforeHead(count) => FollowUpJson::SelectBeforeHead { count: *count },
            FollowUp::OpenCommitPage(commit) => FollowUpJson::OpenCommitPage {
                commit: commit.as_str().to_owned(),
            },
            FollowUp::EmptyCommitMessage => FollowUpJson::EmptyCommitMessage,
            FollowUp::PreparedMessage => FollowUpJson::PreparedMessage,
            FollowUp::FillCommitMessage(message) => FollowUpJson::FillCommitMessage {
                message: message.clone(),
            },
            FollowUp::ReadPullRequests => FollowUpJson::ReadPullRequests,
            FollowUp::Offer(feature, label) => FollowUpJson::Offer {
                feature: feature.0.to_owned(),
                label: text_of(label),
            },
            FollowUp::CopyToClipboard(text) => FollowUpJson::CopyToClipboard { text: text.clone() },
            FollowUp::OpenUrl(url) => FollowUpJson::OpenUrl { url: url.clone() },
            FollowUp::OpenReview(paths) => FollowUpJson::OpenReview { paths: paths.clone() },
            FollowUp::CloseMergePage(path) => FollowUpJson::CloseMergePage { path: path.clone() },
            FollowUp::OpenWindow(folder) => FollowUpJson::OpenWindow {
                folder: path_of(folder),
            },
            FollowUp::ShowFolder(folder) => FollowUpJson::ShowFolder {
                folder: path_of(folder),
            },
            FollowUp::RememberInputs(values) => FollowUpJson::RememberInputs { values: values.clone() },
            FollowUp::EditFile(path, line) => FollowUpJson::EditFile {
                path: path.clone(),
                line: *line,
            },
            FollowUp::OfferRun(offered) => offer_run(offered)?,
            FollowUp::WriteSetting(written) => FollowUpJson::WriteSetting {
                key: written.key.clone(),
                value: match &written.value {
                    Some(text) => Some(serde_json::from_str(text).ok()?),
                    None => None,
                },
            },
            _ => return None,
        })
    }

    /// The follow-ups of a list that are the extension's, in their order.
    #[must_use]
    pub fn all_of(follow_ups: &[FollowUp]) -> Vec<FollowUpJson> {
        follow_ups.iter().filter_map(FollowUpJson::of).collect()
    }
}
