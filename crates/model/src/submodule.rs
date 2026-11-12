//! The submodules of a repository, and where each stands against the commit recorded for it.

use crate::CommitId;

/// A submodule, and theirs, as read: what `.gitmodules` says of it, the commit recorded for it, and what it has
/// checked out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Submodule {
    /// Its name in `.gitmodules`.
    pub name: String,
    /// Its folder from the top of the repository read, with `/`: `vendor/a/b` for a submodule `b` of `vendor/a`.
    pub folder: String,
    /// The folder of the submodule that has it, from the same top; none for one of the repository read.
    pub parent: Option<String>,
    /// The URL `.gitmodules` gives.
    pub url: Option<String>,
    /// The branch `.gitmodules` gives it to follow.
    pub branch: Option<String>,
    /// The commit recorded for it in the last commit; none before the first commit, or when it is added but not
    /// committed yet.
    pub recorded: Option<CommitId>,
    /// The commit staged for it, when it is not the one recorded.
    pub staged: Option<CommitId>,
    /// What its folder has checked out; none when its folder has no repository.
    pub checkout: Option<Checkout>,
}

/// What the repository of a submodule has checked out, and how it stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkout {
    /// The commit checked out; none in a repository with no commit.
    pub commit: Option<CommitId>,
    /// The branch checked out, by its short name; none when detached.
    pub branch: Option<String>,
    /// How many files are not committed in it.
    pub uncommitted: usize,
    /// How far the commit checked out is from the one recorded; none when that is not known, as when the recorded
    /// commit is not in it.
    pub distance: Option<Distance>,
}

/// How far apart the commit a submodule has checked out and the commit recorded for it are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Distance {
    /// How many commits the one checked out has that the recorded one has not.
    pub ahead: u32,
    /// How many commits the recorded one has that the one checked out has not.
    pub behind: u32,
}

/// Where a submodule stands, as its icon and its tooltip say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmoduleStatus {
    /// Its folder has no repository.
    NotInitialized,
    /// No commit is recorded for it yet.
    Added,
    /// The recorded commit is not in it.
    Unknown {
        /// The recorded commit.
        recorded: CommitId,
    },
    /// Each has commits the other has not.
    Both {
        /// How far apart they are.
        distance: Distance,
        /// The recorded commit.
        recorded: CommitId,
    },
    /// It has commits after the recorded one.
    Ahead {
        /// How many.
        commits: u32,
        /// The recorded commit.
        recorded: CommitId,
    },
    /// It is behind the recorded one.
    Behind {
        /// How many.
        commits: u32,
        /// The recorded commit.
        recorded: CommitId,
    },
    /// It has the recorded commit checked out.
    OnIt {
        /// The recorded commit.
        recorded: CommitId,
    },
}

impl SubmoduleStatus {
    /// The sentence of the tooltip, from the card `submodule.status`.
    #[must_use]
    pub fn sentence(&self) -> String {
        match self {
            SubmoduleStatus::NotInitialized => String::from("Not initialized: its folder has no checkout"),
            SubmoduleStatus::Added => String::from("Added, not committed yet"),
            SubmoduleStatus::Unknown { recorded } => format!(
                "The recorded commit {} is not in it: fetch in it, or update it",
                recorded.short()
            ),
            SubmoduleStatus::Both { distance, recorded } => format!(
                "{} ahead of and {} behind the recorded commit {}",
                commits(distance.ahead),
                commits(distance.behind),
                recorded.short()
            ),
            SubmoduleStatus::Ahead {
                commits: count,
                recorded,
            } => {
                format!("{} ahead of the recorded commit {}", commits(*count), recorded.short())
            }
            SubmoduleStatus::Behind {
                commits: count,
                recorded,
            } => {
                format!("{} behind the recorded commit {}", commits(*count), recorded.short())
            }
            SubmoduleStatus::OnIt { recorded } => format!("On the recorded commit {}", recorded.short()),
        }
    }
}

/// `1 commit`, `2 commits`.
fn commits(count: u32) -> String {
    if count == 1 {
        String::from("1 commit")
    } else {
        format!("{count} commits")
    }
}

/// Where a submodule stands, from the commit recorded for it and the commit it has checked out: the first rule of the
/// card that applies.
///
/// Feature: submodule.status
#[must_use]
pub fn status(submodule: &Submodule) -> SubmoduleStatus {
    let Some(checkout) = &submodule.checkout else {
        return SubmoduleStatus::NotInitialized;
    };
    let Some(recorded) = submodule.recorded.clone() else {
        return SubmoduleStatus::Added;
    };
    let distance = if checkout.commit.as_ref() == Some(&recorded) {
        Some(Distance::default())
    } else {
        checkout.distance
    };
    match distance {
        None => SubmoduleStatus::Unknown { recorded },
        Some(distance) if distance.ahead > 0 && distance.behind > 0 => SubmoduleStatus::Both { distance, recorded },
        Some(Distance { ahead, .. }) if ahead > 0 => SubmoduleStatus::Ahead {
            commits: ahead,
            recorded,
        },
        Some(Distance { behind, .. }) if behind > 0 => SubmoduleStatus::Behind {
            commits: behind,
            recorded,
        },
        Some(_) => SubmoduleStatus::OnIt { recorded },
    }
}

#[cfg(test)]
mod tests;
