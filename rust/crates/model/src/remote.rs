//! The remotes of a repository, and what a push to one would change.

/// A remote of the repository, as `remote.read-remotes` reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remote {
    /// Its name: `origin`.
    pub name: String,
    /// The URL it is fetched from, as git uses it.
    pub url: String,
}

/// The commits of an upstream that the branch checked out does not have, as of the last fetch: what a force push
/// throws away (`sync.read-overwritten`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Overwritten {
    /// The first ten of them, newest first, each as its short hash and its subject.
    pub commits: Vec<OverwrittenCommit>,
    /// Whether the upstream has more of them than those.
    pub has_more: bool,
}

/// A commit a force push would throw away.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverwrittenCommit {
    /// Its short hash.
    pub short_id: String,
    /// The first line of its message.
    pub subject: String,
}

/// The remote a new upstream or a tag goes to: `origin` when the repository has a remote of that name, otherwise the
/// first remote git lists, and none when there is no remote.
///
/// Feature: remote.default
#[must_use]
pub fn default(remotes: &[String]) -> Option<String> {
    remotes
        .iter()
        .find(|remote| *remote == "origin")
        .or_else(|| remotes.first())
        .cloned()
}

/// The remote a remote branch belongs to, and the name of the branch on it: the longest name among the remotes that
/// the remote branch starts with, followed by `/`. None when no remote matches.
#[must_use]
pub fn split_remote_branch<'a>(remote_branch: &'a str, remotes: &'a [String]) -> Option<(&'a str, &'a str)> {
    remotes
        .iter()
        .filter_map(|remote| {
            let branch = remote_branch.strip_prefix(remote.as_str())?.strip_prefix('/')?;
            (!branch.is_empty()).then_some((remote.as_str(), branch))
        })
        .max_by_key(|(remote, _)| remote.len())
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{default, split_remote_branch};

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn remote_default__origin_first_then_the_first_listed_then_none() {
        assert_eq!(default(&names(&["upstream", "origin"])).as_deref(), Some("origin"));
        assert_eq!(default(&names(&["a", "b"])).as_deref(), Some("a"));
        assert_eq!(default(&[]), None);
    }

    #[test]
    fn the_longest_remote_a_remote_branch_starts_with_is_its_remote() {
        let remotes = names(&["a", "a/b"]);
        assert_eq!(split_remote_branch("a/b/x", &remotes), Some(("a/b", "x")));
        assert_eq!(split_remote_branch("a/x", &remotes), Some(("a", "x")));
        assert_eq!(split_remote_branch("c/x", &remotes), None);
    }
}
