//! Rules about the names of branches, tags and remotes, what a double click on a branch does, and the format of the
//! names of new branches.

mod format;
mod offers;
mod open;

use crate::contract::Text;
use crate::state::{RepositoryState, Worktree};

pub use format::{BranchInput, FormatValues, InputKind, day_of, format_name, format_problem, repository_name};
pub use offers::{can_fast_forward, rebase_onto};
pub use open::{OpenBranchInput, OpenedReference, open};

/// The local branch of a remote branch's name: the remote branch without its remote, the longest remote name that
/// matches taken off. `origin/feature/login` gives `feature/login`; with remotes `a` and `a/b`, `a/b/x` gives `x`. A
/// name no remote starts is given as it is.
#[must_use]
pub fn local_name<'a>(remote_branch: &'a str, remotes: &[String]) -> &'a str {
    remotes
        .iter()
        .filter_map(|remote| remote_branch.strip_prefix(remote.as_str())?.strip_prefix('/'))
        .min_by_key(|rest| rest.len())
        .unwrap_or(remote_branch)
}

/// The worktree other than the one read that has a local branch checked out, by the branch's short name.
#[must_use]
pub fn other_worktree<'a>(state: &'a RepositoryState, branch: &str) -> Option<&'a Worktree> {
    let full = format!("refs/heads/{branch}");
    state
        .worktrees
        .iter()
        .find(|worktree| !worktree.is_current && worktree.branch.as_deref() == Some(full.as_str()))
}

/// What keeps a name from being a branch, a tag or a remote, said while it is typed; nothing when git would take it.
/// Git has the final say; this catches the common mistakes early.
///
/// Feature: branch.validate-name
#[must_use]
#[allow(
    clippy::case_sensitive_file_extension_comparisons,
    reason = "`.lock` ends a reference name here, not a file name, and git refuses it in exactly this case"
)]
pub fn validate_name(name: &str) -> Option<Text> {
    if name.is_empty() {
        return Some(Text::from("Enter a name"));
    }
    let has_forbidden_character = name
        .chars()
        .any(|character| character.is_whitespace() || "~^:?*[\\".contains(character));
    let is_invalid = name.starts_with(['-', '.', '/'])
        || has_forbidden_character
        || name.contains("..")
        || name.contains("@{")
        || name.ends_with(['/', '.'])
        || name.ends_with(".lock");
    is_invalid.then(|| Text::from("That is not a valid Git ref name"))
}

/// The kinds of references a name can be of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefKind {
    /// A local branch, under `refs/heads/`.
    Branch,
    /// A remote branch, under `refs/remotes/`.
    RemoteBranch,
    /// A tag, under `refs/tags/`.
    Tag,
}

impl RefKind {
    /// What the full name of a reference of this kind starts with.
    #[must_use]
    pub fn prefix(self) -> &'static str {
        match self {
            RefKind::Branch => "refs/heads/",
            RefKind::RemoteBranch => "refs/remotes/",
            RefKind::Tag => "refs/tags/",
        }
    }
}

/// The short names of the references of a repository, by kind.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RefNames {
    /// The local branches: `main`.
    pub branches: Vec<String>,
    /// The remote branches: `origin/main`.
    pub remote_branches: Vec<String>,
    /// The tags: `v1.0.0`.
    pub tags: Vec<String>,
}

impl RefNames {
    /// The names of the references of a repository as read.
    #[must_use]
    pub fn of(state: &RepositoryState) -> Self {
        RefNames {
            branches: state.branches.iter().map(|branch| branch.name.clone()).collect(),
            remote_branches: state.remote_branches.iter().map(|branch| branch.name.clone()).collect(),
            tags: state.tags.iter().map(|tag| tag.name.clone()).collect(),
        }
    }
}

/// How a branch, a remote branch or a tag is named to git where git expects a commit: in full when another kind of
/// reference has the same name, since git would take the tag for a branch of its name; otherwise by its short name.
///
/// Feature: branch.ref-argument
#[must_use]
pub fn ref_argument(kind: RefKind, name: &str, refs: &RefNames) -> String {
    let uses = [&refs.branches, &refs.remote_branches, &refs.tags]
        .iter()
        .filter(|names| names.iter().any(|known| known == name))
        .count();
    if uses > 1 {
        format!("{}{name}", kind.prefix())
    } else {
        name.to_owned()
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{RefKind, RefNames, ref_argument, validate_name};

    fn names(branches: &[&str], remote_branches: &[&str], tags: &[&str]) -> RefNames {
        let owned = |list: &[&str]| list.iter().map(|name| (*name).to_owned()).collect();
        RefNames {
            branches: owned(branches),
            remote_branches: owned(remote_branches),
            tags: owned(tags),
        }
    }

    #[test]
    fn branch_validate_name__takes_ordinary_names() {
        for name in ["feature/login", "v1.0.0", "fix-123"] {
            assert_eq!(validate_name(name), None, "{name}");
        }
    }

    #[test]
    fn branch_validate_name__refuses_what_git_does_not_take() {
        for name in [
            "-x", ".x", "a..b", "a b", "a:b", "a@{1}", "x/", "x.lock", "/x", "a~1", "a^", "a?", "a*", "a[", "a\\b",
            "x.",
        ] {
            assert_eq!(
                validate_name(name).map(|text| text.to_string()),
                Some(String::from("That is not a valid Git ref name")),
                "{name}"
            );
        }
    }

    #[test]
    fn branch_validate_name__asks_for_a_name_when_empty() {
        assert_eq!(
            validate_name("").map(|text| text.to_string()),
            Some(String::from("Enter a name"))
        );
    }

    #[test]
    fn branch_ref_argument__names_a_branch_and_a_tag_of_the_same_name_in_full() {
        let refs = names(&["release", "main"], &[], &["release"]);
        assert_eq!(ref_argument(RefKind::Branch, "release", &refs), "refs/heads/release");
        assert_eq!(ref_argument(RefKind::Tag, "release", &refs), "refs/tags/release");
    }

    #[test]
    fn branch_ref_argument__names_a_branch_by_its_short_name_when_no_other_has_it() {
        let refs = names(&["main"], &["origin/main"], &[]);
        assert_eq!(ref_argument(RefKind::Branch, "main", &refs), "main");
        assert_eq!(ref_argument(RefKind::RemoteBranch, "origin/main", &refs), "origin/main");
    }

    #[test]
    fn branch_ref_argument__pushes_a_branch_that_has_a_tag_of_its_name_in_full() {
        // `sync.push` gives git the branch as this function names it: `git push origin refs/heads/release`.
        let refs = names(&["release"], &[], &["release"]);
        let pushed = ref_argument(RefKind::Branch, "release", &refs);
        assert_eq!(
            ["push", "origin", pushed.as_str()],
            ["push", "origin", "refs/heads/release"]
        );
    }
}
