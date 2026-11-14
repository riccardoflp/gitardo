//! The words of an operation stopped on conflicts: the banner above the commits and the Changes page, the message when
//! an operation stops, and the buttons of a file in conflict (`conflicts.`).

use model::state::Operation;

/// **Continue** in the banner (`conflicts.continue-operation`).
pub const CONTINUE: &str = "Continue";
/// The tooltip of **Continue** while files are in conflict.
pub const RESOLVE_FIRST: &str = "Resolve the conflicts and stage the files first";
/// **Skip** in the banner (`conflicts.skip-step`).
pub const SKIP: &str = "Skip";
/// **Abort** in the banner (`conflicts.abort-operation`).
pub const ABORT: &str = "Abort";
/// **Resolve with AI** in the banner and the message (`ai.resolve-conflicts`).
pub const RESOLVE_WITH_AI: &str = "Resolve with AI";
/// **Resolve automatically** in the message (`conflicts.resolve-automatically`).
pub const RESOLVE_AUTOMATICALLY: &str = "Resolve automatically";
/// **Show log** in the message (`log.show`).
pub const SHOW_LOG: &str = "Show log";
/// The tooltip of the button that resolves the conflicts that need no choice.
pub const RESOLVE_AUTOMATICALLY_TOOLTIP: &str = "Resolve the conflicts that need no choice between the two sides: \
     changes to lines next to each other, the same change made by both, differences in spaces or line endings only. \
     Gitardo shows each resolution first, to accept all of them or one by one; the other conflicts stay as they are.";
/// What the screen reader calls the banner.
pub const BANNER: &str = "Operation stopped";

/// The operation, as the banner names it: **Merge**, **Rebase**, **Cherry-pick**, **Revert**, **Patch (git am)**.
#[must_use]
pub fn operation_name(operation: Operation) -> &'static str {
    match operation {
        Operation::Merge => "Merge",
        Operation::Rebase => "Rebase",
        Operation::CherryPick => "Cherry-pick",
        Operation::Revert => "Revert",
        Operation::Patch => "Patch (git am)",
    }
}

/// How many files are in conflict: *1 file has conflicts*, *2 files have conflicts*.
#[must_use]
pub fn files_have_conflicts(count: usize) -> String {
    if count == 1 {
        String::from("1 file has conflicts")
    } else {
        format!("{count} files have conflicts")
    }
}

/// What the banner says: **Merge in progress · 2 files have conflicts**, **Merge in progress**, **1 file has
/// conflicts**.
#[must_use]
pub fn banner(operation: Option<Operation>, conflicted: usize) -> String {
    match (operation, conflicted) {
        (Some(operation), 0) => format!("{} in progress", operation_name(operation)),
        (Some(operation), count) => format!(
            "{} in progress \u{b7} {}",
            operation_name(operation),
            files_have_conflicts(count)
        ),
        (None, count) => files_have_conflicts(count),
    }
}

/// The button that resolves the conflicts that need no choice: **Resolve it automatically** for a single conflict,
/// **Resolve all 3 automatically**, **Resolve 2 of 3 automatically**.
#[must_use]
pub fn resolve_automatically(resolvable: usize, conflicts: usize) -> String {
    match (resolvable, conflicts) {
        (1, 1) => String::from("Resolve it automatically"),
        (resolvable, conflicts) if resolvable == conflicts => format!("Resolve all {conflicts} automatically"),
        (resolvable, conflicts) => format!("Resolve {resolvable} of {conflicts} automatically"),
    }
}

/// The message when an operation stops on conflicts: *Merge feature into main stopped: 2 files have conflicts. Resolve
/// them, then continue or abort the merge.*, or *then stage the files.* with no operation left to continue.
#[must_use]
pub fn stopped(title: &str, conflicted: usize, operation: Option<&str>) -> String {
    let then = match operation {
        Some(operation) => format!("then continue or abort the {operation}."),
        None => String::from("then stage the files."),
    };
    format!(
        "{title} stopped: {}. Resolve them, {then}",
        files_have_conflicts(conflicted)
    )
}

/// What the message adds when the rules resolve some conflicts: *Gitardo can resolve all 3 conflicts automatically.*,
/// *the conflict* for one, *2 of the 3 conflicts* for some.
#[must_use]
pub fn can_resolve(resolvable: usize, conflicts: usize) -> String {
    let which = match (resolvable, conflicts) {
        (1, 1) => String::from("the conflict"),
        (resolvable, conflicts) if resolvable == conflicts => format!("all {conflicts} conflicts"),
        (resolvable, conflicts) => format!("{resolvable} of the {conflicts} conflicts"),
    };
    format!("Gitardo can resolve {which} automatically.")
}

/// The title of the Diff page of a file one side deleted: **app.rs (deleted by the branch being merged, changed by the
/// branch checked out)**.
#[must_use]
pub fn deleted_and_changed(name: &str, deleter: &str, keeper: &str) -> String {
    format!("{name} (deleted by {deleter}, changed by {keeper})")
}

/// The operation, inside a sentence: `merge`, `rebase`, `cherry-pick`, `revert`, `patch`.
#[must_use]
pub fn operation_noun(operation: Operation) -> &'static str {
    match operation {
        Operation::Merge => "merge",
        Operation::Rebase => "rebase",
        Operation::CherryPick => "cherry-pick",
        Operation::Revert => "revert",
        Operation::Patch => "patch",
    }
}
