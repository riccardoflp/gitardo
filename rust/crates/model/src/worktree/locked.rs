//! `worktree.locked`: what a locked worktree shows, and what **Remove…**, **Rename…** and **Prune** do with it.

use crate::state::Worktree;

/// What the tooltip of a locked worktree says before its reason.
pub const LOCKED: &str = "Locked";
/// What the row of a locked worktree whose folder is gone says.
pub const MISSING_LOCKED: &str = "missing, locked";

/// What [`locked`] looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockedInput<'a> {
    /// How the views name the worktree.
    pub label: &'a str,
    /// The reason it is locked, empty when none was given; none when it is not locked.
    pub reason: Option<&'a str>,
    /// Whether its folder is gone.
    pub is_missing: bool,
}

/// What a lock changes for a worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedRules {
    /// The tooltip of its lock, **Locked: `<reason>`** or **Locked**; none when it is not locked.
    pub tooltip: Option<String>,
    /// What its row says of it when its folder is gone and it is locked: `missing, locked`.
    pub standing: Option<&'static str>,
    /// What **Remove…** and **Rename…** ask first: **`<label>` is locked: `<reason>`.**; none when it is not locked.
    pub ask: Option<String>,
    /// Whether `git worktree prune` would forget it: missing and not locked.
    pub is_prunable: bool,
}

/// The rules of `worktree.locked` for one worktree.
///
/// Feature: worktree.locked
#[must_use]
pub fn locked(input: &LockedInput<'_>) -> LockedRules {
    let reason = input.reason.map(str::trim);
    LockedRules {
        tooltip: reason.map(|reason| match reason {
            "" => LOCKED.to_owned(),
            reason => format!("{LOCKED}: {reason}"),
        }),
        standing: (reason.is_some() && input.is_missing).then_some(MISSING_LOCKED),
        ask: reason.map(|reason| match reason {
            "" => format!("{} is locked.", input.label),
            reason => format!("{} is locked: {}.", input.label, reason.trim_end_matches('.')),
        }),
        is_prunable: input.is_missing && reason.is_none(),
    }
}

/// Whether **Prune missing worktrees** has something to forget: a worktree missing and not locked.
#[must_use]
pub fn has_prunable(worktrees: &[Worktree]) -> bool {
    worktrees
        .iter()
        .any(|worktree| worktree.is_missing && worktree.locked.is_none())
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{LockedInput, locked};

    fn rules(reason: Option<&str>, is_missing: bool) -> super::LockedRules {
        locked(&LockedInput {
            label: "feature/login",
            reason,
            is_missing,
        })
    }

    #[test]
    fn worktree_locked__a_reason_is_said_in_the_tooltip_and_in_the_ask() {
        let rules = rules(Some("On the USB drive"), false);
        assert_eq!(rules.tooltip.as_deref(), Some("Locked: On the USB drive"));
        assert_eq!(rules.ask.as_deref(), Some("feature/login is locked: On the USB drive."));
        assert_eq!(rules.standing, None);
    }

    #[test]
    fn worktree_locked__a_lock_with_no_reason_says_locked() {
        let rules = rules(Some(""), false);
        assert_eq!(rules.tooltip.as_deref(), Some("Locked"));
        assert_eq!(rules.ask.as_deref(), Some("feature/login is locked."));
    }

    #[test]
    fn worktree_locked__a_locked_worktree_missing_says_missing_locked_and_is_not_prunable() {
        let rules = rules(Some("On the USB drive"), true);
        assert_eq!(rules.standing, Some("missing, locked"));
        assert!(!rules.is_prunable);
        assert!(
            super::super::locked(&LockedInput {
                label: "a",
                reason: None,
                is_missing: true
            })
            .is_prunable
        );
    }

    #[test]
    fn worktree_locked__a_worktree_not_locked_has_no_lock() {
        let rules = rules(None, false);
        assert_eq!((rules.tooltip, rules.ask, rules.standing), (None, None, None));
    }
}
