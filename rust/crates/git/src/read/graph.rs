//! The commits the graph lists, a page at a time, and where a commit is in them.
//!
//! Each read's arguments and parser are apart and public (rule R2.5), so that the golden tests record exactly what the
//! read asks git and parse it without git.

mod commits;
mod position;

use std::ffi::OsString;

pub use commits::{
    CommitPage, CommitsRequest, commits_arguments, parse_stash_commits, read_commits, stashes_arguments,
};
pub use position::{PositionRequest, parse_position, position_arguments, read_position};

/// Which history the graph lists.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct History {
    /// The arguments that say which commits are listed and in which order, as `filter.log-arguments` gives them:
    /// `--date-order` first, then the branches shown.
    pub selection: Vec<OsString>,
    /// What narrows the history further, after the selection: the arguments of a search shown as a list, or of the
    /// history of a file, a folder or some lines, which end with `--` and the paths when they have paths.
    pub narrowing: Vec<OsString>,
    /// Which parents of each commit are followed: `graph.firstParent`.
    pub parents: Parents,
}

/// Which parents of each commit the graph follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Parents {
    /// All of them.
    #[default]
    All,
    /// Only the first, as `graph.firstParent` asks: a merge is one commit on its branch's line.
    First,
}

impl History {
    /// The arguments of `git log` after its format and its count: `--first-parent` when only the first parents are
    /// followed, the selection, the narrowing, and `--` when the narrowing does not end the options itself, so that a
    /// branch is never taken for a file of the same name.
    #[must_use]
    pub fn arguments(&self) -> Vec<OsString> {
        let mut arguments = Vec::with_capacity(self.selection.len() + self.narrowing.len() + 2);
        if self.parents == Parents::First {
            arguments.push(OsString::from("--first-parent"));
        }
        arguments.extend(self.selection.iter().cloned());
        arguments.extend(self.narrowing.iter().cloned());
        if !self.narrowing.iter().any(|argument| argument == "--") {
            arguments.push(OsString::from("--"));
        }
        arguments
    }
}
