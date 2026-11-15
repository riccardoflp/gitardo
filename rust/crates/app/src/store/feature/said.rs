//! What is said once an operation ended, over the window.

use model::CommitId;
use model::contract::FeatureId;

/// What was said once an operation ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Said {
    /// The sentence.
    pub text: String,
    /// Whether it says the operation failed or was refused.
    pub is_error: bool,
    /// Whether **Undo** is offered beside it.
    pub can_undo: bool,
    /// Whether it warns of a stash a pull left, with **Show Stashes** beside it (`stash.autostash-left`).
    pub offers_stashes: bool,
    /// The first bad commit a bisect found, named with **Show in the timeline** and **End bisect** beside it.
    pub found: Option<CommitId>,
    /// What it offers to do next, with the words of its button: **Continue** once no file is left in conflict.
    pub offer: Option<(FeatureId, String)>,
    /// The title of the operation that stopped on conflicts and how many files were in conflict, once the repository
    /// read again has some: the message of `conflicts.stopped` is shown in place of the failure.
    pub stopped: Option<(String, usize)>,
    /// The operations offered beside it, each with its button: **Open in new window**, **Remove anyway**.
    pub runs: Vec<model::contract::OfferedRun>,
    /// The words of a button that opens an address in the browser, and the address: **Create pull request** after the
    /// first push of a branch (`github.offer-after-push`).
    pub link: Option<(String, String)>,
}

impl Said {
    /// A failure said alone, with nothing beside it: a new window that could not be opened.
    #[must_use]
    pub fn error(text: String) -> Said {
        Said::new(text, true)
    }

    /// A sentence alone, with nothing beside it.
    pub(in crate::store) fn new(text: String, is_error: bool) -> Said {
        Said {
            text,
            is_error,
            can_undo: false,
            offers_stashes: false,
            found: None,
            offer: None,
            stopped: None,
            runs: Vec::new(),
            link: None,
        }
    }
}
