//! The JSON of the follow-ups that are the extension's to carry out once a feature ran.

use std::path::Path;

use model::contract::{FollowUp, Text};
use serde::{Deserialize, Serialize};

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
    /// An operation is offered with its button, on a target and with answers given before it asks: **Open in new
    /// window** once a worktree is made, **Remove anyway** once git refused to remove one.
    OfferRun {
        /// The words of the button.
        label: String,
        /// The feature.
        feature: String,
        /// What it runs on.
        target: super::question::TargetJson,
        /// The answers it is given, by the key of each question.
        answers: std::collections::BTreeMap<String, super::question::AnswerJson>,
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
        values: std::collections::BTreeMap<String, String>,
    },
    /// A file opens in an editor at a line.
    EditFile {
        /// The file.
        path: String,
        /// The line, from 1.
        line: usize,
    },
}

fn text_of(text: &Text) -> String {
    text.as_str().to_owned()
}

fn path_of(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// The JSON of an operation offered; none when its target or one of its answers does not cross yet.
fn offer_run(offer: &model::contract::OfferedRun) -> Option<FollowUpJson> {
    let mut answers = std::collections::BTreeMap::new();
    for (key, answer) in offer.answers.iter() {
        let _given = answers.insert(key.0.to_owned(), super::question::AnswerJson::of(answer)?);
    }
    Some(FollowUpJson::OfferRun {
        label: text_of(&offer.label),
        feature: offer.feature.0.to_owned(),
        target: super::question::TargetJson::of(&offer.target)?,
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
            FollowUp::OfferRun(offer) => offer_run(offer)?,
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
            _ => return None,
        })
    }

    /// The follow-ups of a list that are the extension's, in their order.
    #[must_use]
    pub fn all_of(follow_ups: &[FollowUp]) -> Vec<FollowUpJson> {
        follow_ups.iter().filter_map(FollowUpJson::of).collect()
    }
}
