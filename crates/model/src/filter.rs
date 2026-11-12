//! The branches the timeline shows: everything, a preset, some kinds of references, or references ticked one by one;
//! what each choice is called, which references it stands for, how it changes, which arguments of `git log` it gives,
//! and how it is kept for each repository.

mod choice;
mod label;
mod log;
mod rank;
mod remember;

use crate::refs::References;

pub use choice::{hide, kinds, kinds_shown, preset, show_all, show_only, show_too, tick, ticked};
pub use label::{label, label_parts};
pub use log::log_arguments;
pub use rank::rank_names;
pub use remember::{MOST_TICKED, StoredValue, checked, remember};

/// What the timeline shows when it is narrowed; `None` beside it, in the functions here, is everything.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BranchesShown {
    /// **Current**: what is checked out, whichever branch that is as you switch.
    Current,
    /// **Upstream**: what is checked out and the upstream of its branch.
    Upstream,
    /// Some kinds of references and what is checked out, never all three, which is everything: **Local** is the local
    /// branches alone, **Remotes** the remote branches alone.
    Kinds(Kinds),
    /// References ticked one by one, by their full names (`refs/heads/main`, `refs/remotes/origin/main`,
    /// `refs/tags/v1.0`), in the order they were ticked.
    References(Vec<String>),
}

/// A kind of reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReferenceKind {
    /// A local branch, under `refs/heads/`.
    Local,
    /// A remote branch, under `refs/remotes/`.
    Remote,
    /// A tag, under `refs/tags/`.
    Tag,
}

impl ReferenceKind {
    /// Every kind, in the order the lists and the menus name them.
    pub const ALL: [ReferenceKind; 3] = [ReferenceKind::Local, ReferenceKind::Remote, ReferenceKind::Tag];

    /// Where the references of the kind are: `refs/heads/`, `refs/remotes/`, `refs/tags/`.
    #[must_use]
    pub fn prefix(self) -> &'static str {
        match self {
            ReferenceKind::Local => "refs/heads/",
            ReferenceKind::Remote => "refs/remotes/",
            ReferenceKind::Tag => "refs/tags/",
        }
    }

    /// The kind of a reference by its full name, when it is a branch, a remote branch or a tag.
    #[must_use]
    pub fn of(full_name: &str) -> Option<ReferenceKind> {
        ReferenceKind::ALL
            .into_iter()
            .find(|kind| full_name.starts_with(kind.prefix()))
    }

    /// The full name of a reference of the kind, from its short name: `main` is `refs/heads/main` as a branch.
    #[must_use]
    pub fn full_name(self, short_name: &str) -> String {
        format!("{}{short_name}", self.prefix())
    }
}

/// The kinds of references shown, as one value that is copied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Kinds {
    /// The local branches.
    pub local: bool,
    /// The remote branches.
    pub remote: bool,
    /// The tags.
    pub tags: bool,
}

impl Kinds {
    /// All three kinds: everything.
    pub const ALL: Kinds = Kinds {
        local: true,
        remote: true,
        tags: true,
    };

    /// One kind alone.
    #[must_use]
    pub fn only(kind: ReferenceKind) -> Kinds {
        Kinds::default().with(kind, true)
    }

    /// Whether a kind is among them.
    #[must_use]
    pub fn has(self, kind: ReferenceKind) -> bool {
        match kind {
            ReferenceKind::Local => self.local,
            ReferenceKind::Remote => self.remote,
            ReferenceKind::Tag => self.tags,
        }
    }

    /// The kinds with one added or taken away.
    #[must_use]
    pub fn with(self, kind: ReferenceKind, is_shown: bool) -> Kinds {
        match kind {
            ReferenceKind::Local => Kinds {
                local: is_shown,
                ..self
            },
            ReferenceKind::Remote => Kinds {
                remote: is_shown,
                ..self
            },
            ReferenceKind::Tag => Kinds { tags: is_shown, ..self },
        }
    }

    /// How many kinds there are.
    #[must_use]
    pub fn count(self) -> usize {
        ReferenceKind::ALL.into_iter().filter(|kind| self.has(*kind)).count()
    }
}

/// A choice of one click at the top of the branch filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Preset {
    /// Everything: nothing narrows the timeline.
    All,
    /// What is checked out.
    Current,
    /// What is checked out and the upstream of its branch.
    Upstream,
    /// The local branches, and what is checked out.
    Local,
    /// The remote branches, and what is checked out.
    Remotes,
}

impl Preset {
    /// Every preset, in the order of the branch filter.
    pub const ALL: [Preset; 5] = [
        Preset::All,
        Preset::Current,
        Preset::Upstream,
        Preset::Local,
        Preset::Remotes,
    ];
}

/// The short name of a reference by its full name: `refs/remotes/origin/main` is `origin/main`; a name under none of
/// the three places is kept whole.
#[must_use]
pub fn short_name(full_name: &str) -> &str {
    ReferenceKind::of(full_name)
        .and_then(|kind| full_name.strip_prefix(kind.prefix()))
        .unwrap_or(full_name)
}

/// The full names of every reference of some kinds, local branches first, then remote branches, then tags, each in
/// the order of the lists.
#[must_use]
pub fn full_names(references: &References, kinds: Kinds) -> Vec<String> {
    let mut names = Vec::new();
    if kinds.local {
        names.extend(
            references
                .branches
                .iter()
                .map(|branch| ReferenceKind::Local.full_name(&branch.name)),
        );
    }
    if kinds.remote {
        names.extend(
            references
                .remote_branches
                .iter()
                .map(|branch| ReferenceKind::Remote.full_name(&branch.name)),
        );
    }
    if kinds.tags {
        names.extend(
            references
                .tags
                .iter()
                .map(|tag| ReferenceKind::Tag.full_name(&tag.name)),
        );
    }
    names
}

#[cfg(test)]
pub(crate) mod testing {
    use crate::Timestamp;
    use crate::graph::testing::{branch, id, remote_branch};
    use crate::refs::References;
    use crate::state::Tag;

    /// References of a repository: local branches (`main` checked out, tracking `origin/main`), remote branches and
    /// tags, all at one commit.
    pub(crate) fn references(local: &[&str], remote: &[&str], tags: &[&str]) -> References {
        References {
            branches: local
                .iter()
                .map(|name| {
                    let upstream = (*name == "main").then_some(("origin", "main"));
                    let mut made = branch(name, "A", upstream);
                    made.is_checked_out = *name == "main";
                    made
                })
                .collect(),
            remote_branches: remote.iter().map(|name| remote_branch(name, "A")).collect(),
            tags: tags
                .iter()
                .map(|name| Tag {
                    name: (*name).to_owned(),
                    commit: id("A"),
                    date: Timestamp(0),
                })
                .collect(),
        }
    }
}
