//! What the branch rail lists from the choice of its own filter (`refs.listed`), apart from what the timeline shows: the
//! same presets, kinds and ticks as the timeline's branch filter, over the rail alone.

use std::collections::HashSet;

use super::{BranchesShown, StoredValue, full_names, remember, ticked};
use crate::refs::References;

/// The choice of the filter of the branch rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Choice<'a> {
    /// What the rail lists; none is everything.
    pub listed: Option<&'a BranchesShown>,
}

/// The branches, remote branches and tags the branch rail lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Listed {
    /// Every one of them.
    Everything,
    /// Only these, by their full names.
    Only(HashSet<String>),
}

impl Listed {
    /// Whether the rail lists a reference, by its full name.
    #[must_use]
    pub fn has(&self, full_name: &str) -> bool {
        match self {
            Listed::Everything => true,
            Listed::Only(names) => names.contains(full_name),
        }
    }
}

/// What the branch rail lists from the choice of its own filter: **All** everything; **Current** and **Upstream** the
/// branch checked out and its upstream; some kinds every reference of those kinds; references ticked one by one those
/// that still exist, and everything once none does, as the timeline's choice kept (`filter.remember`). Stashes,
/// worktrees and submodules are not references: the rail always lists them.
///
/// Feature: refs.listed
#[must_use]
pub fn listed(choice: &Choice<'_>, references: &References) -> Listed {
    let names = match choice.listed {
        None => return Listed::Everything,
        Some(BranchesShown::Current | BranchesShown::Upstream) => ticked(Some(&BranchesShown::Upstream), references),
        Some(BranchesShown::Kinds(kinds)) => full_names(references, *kinds),
        Some(ticked_one_by_one @ BranchesShown::References(_)) => {
            match remember(&StoredValue::of(ticked_one_by_one), references) {
                Some(BranchesShown::References(names)) => names,
                _ => return Listed::Everything,
            }
        }
    };
    Listed::Only(names.into_iter().collect())
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
