//! How the choice of the branches shown changes: a preset, a tick, a kind switched, a reference shown alone, added,
//! hidden, or everything shown again.

use super::{BranchesShown, Kinds, Preset, ReferenceKind, full_names};
use crate::refs::References;

/// What a preset of the branch filter shows: none is everything.
///
/// Feature: filter.preset
#[must_use]
pub fn preset(preset: Preset) -> Option<BranchesShown> {
    match preset {
        Preset::All => None,
        Preset::Current => Some(BranchesShown::Current),
        Preset::Upstream => Some(BranchesShown::Upstream),
        Preset::Local => Some(BranchesShown::Kinds(Kinds::only(ReferenceKind::Local))),
        Preset::Remotes => Some(BranchesShown::Kinds(Kinds::only(ReferenceKind::Remote))),
    }
}

/// The references a choice stands for, by their full names: with everything, none; with some kinds, every reference of
/// those kinds; with **Current**, the branch checked out, none when detached; with **Upstream**, that branch and its
/// upstream; with references ticked one by one, those.
#[must_use]
pub fn ticked(shown: Option<&BranchesShown>, references: &References) -> Vec<String> {
    let checked_out = references.branches.iter().find(|branch| branch.is_checked_out);
    match shown {
        None => Vec::new(),
        Some(BranchesShown::Kinds(kinds)) => full_names(references, *kinds),
        Some(BranchesShown::Current) => checked_out
            .map(|branch| ReferenceKind::Local.full_name(&branch.name))
            .into_iter()
            .collect(),
        Some(BranchesShown::Upstream) => checked_out.map_or_default(|branch| {
            let upstream = branch.upstream.as_ref().map(|upstream| upstream.reference.clone());
            std::iter::once(ReferenceKind::Local.full_name(&branch.name))
                .chain(upstream)
                .collect()
        }),
        Some(BranchesShown::References(names)) => names.clone(),
    }
}

/// A reference ticked, or unticked when it is ticked: from a preset or some kinds, the references it stands for with
/// that one added or taken away, ticked one by one. With none left, everything is shown.
///
/// Feature: filter.tick
#[must_use]
pub fn tick(shown: Option<&BranchesShown>, reference: &str, references: &References) -> Option<BranchesShown> {
    let mut names = ticked(shown, references);
    if let Some(index) = names.iter().position(|name| name == reference) {
        let _unticked = names.remove(index);
    } else {
        names.push(reference.to_owned());
    }
    (!names.is_empty()).then_some(BranchesShown::References(names))
}

/// The kinds of references the timeline shows, as the **…** menu checks them; none while the choice is not of kinds
/// (references ticked one by one, **Current** or **Upstream**), when the entries are greyed out.
#[must_use]
pub fn kinds_shown(shown: Option<&BranchesShown>) -> Option<Kinds> {
    match shown {
        None => Some(Kinds::ALL),
        Some(BranchesShown::Kinds(kinds)) => Some(*kinds),
        Some(BranchesShown::Current | BranchesShown::Upstream | BranchesShown::References(_)) => None,
    }
}

/// A kind of reference switched on or off: all three is everything, and one of them always stays on. While the choice
/// is not of kinds, nothing changes.
///
/// Feature: filter.kinds
#[must_use]
pub fn kinds(shown: Option<&BranchesShown>, kind: ReferenceKind) -> Option<BranchesShown> {
    let Some(before) = kinds_shown(shown) else {
        return shown.cloned();
    };
    let after = before.with(kind, !before.has(kind));
    match after.count() {
        0 => shown.cloned(),
        3 => None,
        _ => Some(BranchesShown::Kinds(after)),
    }
}

/// The history of one reference alone, ticked one by one, whatever was shown before.
///
/// Feature: filter.show-only
#[must_use]
pub fn show_only(reference: &str) -> BranchesShown {
    BranchesShown::References(vec![reference.to_owned()])
}

/// A reference added to those shown: the references the choice stands for and this one, ticked one by one.
///
/// Feature: filter.show-too
#[must_use]
pub fn show_too(shown: &BranchesShown, reference: &str, references: &References) -> BranchesShown {
    let mut names = ticked(Some(shown), references);
    if !names.iter().any(|name| name == reference) {
        names.push(reference.to_owned());
    }
    BranchesShown::References(names)
}

/// A reference taken out of those shown: the references the choice stands for but this one, ticked one by one. It is
/// not offered on the only one shown, which would leave nothing; then the choice stays.
///
/// Feature: filter.hide
#[must_use]
pub fn hide(shown: &BranchesShown, reference: &str, references: &References) -> BranchesShown {
    let names: Vec<String> = ticked(Some(shown), references)
        .into_iter()
        .filter(|name| name != reference)
        .collect();
    if names.is_empty() {
        shown.clone()
    } else {
        BranchesShown::References(names)
    }
}

/// Everything shown again.
///
/// Feature: filter.show-all
#[must_use]
pub fn show_all() -> Option<BranchesShown> {
    None
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
