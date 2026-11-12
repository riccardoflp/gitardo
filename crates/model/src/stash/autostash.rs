//! The stash a pull with `--autostash` leaves when the changes it set aside no longer fit on what came in.

use crate::contract::Text;
use crate::state::Stash;

/// How a pull ran, and the stashes before and after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PullOutcome<'a> {
    /// Whether it was run with `--autostash`.
    pub had_autostash: bool,
    /// Whether it succeeded.
    pub succeeded: bool,
    /// The stashes before it, newest first.
    pub stashes_before: &'a [Stash],
    /// The stashes after it, newest first.
    pub stashes_after: &'a [Stash],
}

/// A warning said once an operation ended, with **Show Stashes**, which shows the Stashes list of the branch rail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// The sentence.
    pub message: Text,
}

/// The warning that the changes a pull set aside are in conflict in the files and kept in a stash, when the pull had
/// `--autostash`, succeeded, and left one stash more than before whose message says `autostash`; nothing otherwise, as
/// git dropped the stash or will put the changes back itself once the merge or the rebase is over.
///
/// Feature: stash.autostash-left
#[must_use]
pub fn autostash_left(input: &PullOutcome<'_>) -> Option<Notice> {
    if !input.had_autostash || !input.succeeded || input.stashes_after.len() != input.stashes_before.len() + 1 {
        return None;
    }
    let newest = input.stashes_after.first()?;
    if !newest.message.to_lowercase().contains("autostash") {
        return None;
    }
    Some(Notice {
        message: Text::from(format!(
            "Pulled, but your changes do not fit on what came in. They are in conflict in the files, and whole in the \
             stash \"{}\": resolve the conflicts, then drop the stash.",
            newest.message
        )),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{PullOutcome, autostash_left};
    use crate::state::Stash;
    use crate::{CommitId, Timestamp};

    fn stash(index: usize, message: &str) -> Stash {
        Stash {
            commit: CommitId::parse(&"a".repeat(40)).unwrap(),
            index,
            message: message.to_owned(),
            date: Timestamp(0),
        }
    }

    fn outcome<'a>(had_autostash: bool, before: &'a [Stash], after: &'a [Stash]) -> PullOutcome<'a> {
        PullOutcome {
            had_autostash,
            succeeded: true,
            stashes_before: before,
            stashes_after: after,
        }
    }

    #[test]
    fn stash_autostash_left__a_stash_named_autostash_left_by_the_pull_is_warned_of() {
        let before = [stash(0, "On main: mine")];
        let after = [stash(0, "autostash"), stash(1, "On main: mine")];
        let notice = autostash_left(&outcome(true, &before, &after)).unwrap();
        assert_eq!(
            notice.message.as_str(),
            "Pulled, but your changes do not fit on what came in. They are in conflict in the files, and whole in the \
             stash \"autostash\": resolve the conflicts, then drop the stash."
        );
    }

    #[test]
    fn stash_autostash_left__a_pull_whose_changes_went_back_says_nothing() {
        let before = [stash(0, "On main: mine")];
        assert_eq!(autostash_left(&outcome(true, &before, &before)), None);
        let mine = [stash(0, "On main: mine"), stash(1, "On main: older")];
        assert_eq!(autostash_left(&outcome(true, &before[..0], &mine[..1])), None);
        let mut failed = outcome(true, &[], &mine[..1]);
        failed.succeeded = false;
        assert_eq!(autostash_left(&failed), None);
    }

    #[test]
    fn stash_autostash_left__a_pull_without_autostash_never_warns() {
        let after = [stash(0, "autostash")];
        assert_eq!(autostash_left(&outcome(false, &[], &after)), None);
    }
}
