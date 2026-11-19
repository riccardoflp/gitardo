//! Which branches of a stack move with a rewrite, and what its plan says of them (`rebase.update-refs`).

use std::path::Path;

use super::stacked::{GitVersion, StackedBranches, UPDATE_REFS_VERSION};

/// What decides it.
#[derive(Debug, Clone, Copy)]
pub struct UpdateRefsInput<'a> {
    /// The version of git, none when it could not be read.
    pub version: Option<&'a GitVersion>,
    /// Gitardo's `rebase.updateRefs`, none when it is not set.
    pub setting: Option<bool>,
    /// The branches on the commits made again, and git's own setting.
    pub stacked: &'a StackedBranches,
    /// The folder of the worktree the rebase runs in.
    pub worktree: &'a Path,
}

/// What the rewrite does with the branches of the stack.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateRefs {
    /// `--update-refs` or `--no-update-refs` for a rebase with git's own todo list, or none.
    pub option: Option<&'static str>,
    /// The branches that move with it.
    pub moving: Vec<String>,
    /// The branches left on the old commits.
    pub staying: Vec<String>,
    /// What the plan says of them, after its own words; none when there is no branch.
    pub note: Option<String>,
}

/// Decides whether the other branches of a stack move with a rewrite: when Gitardo's setting is `true`, or when it is
/// not set and git's is not `false`; never a branch checked out in another worktree, nor with a git older than 2.38.
/// The note names the branches that move and says why the others stay.
///
/// Feature: rebase.update-refs
#[must_use]
pub fn update_refs(input: &UpdateRefsInput<'_>) -> UpdateRefs {
    let stacked = input.stacked;
    if stacked.branches.is_empty() {
        return UpdateRefs::default();
    }
    let wanted = input.setting.unwrap_or(stacked.update_refs != Some(false));
    let can = input
        .version
        .is_some_and(|version| version.is_at_least(UPDATE_REFS_VERSION.version));
    let (elsewhere, here): (Vec<_>, Vec<_>) = stacked.branches.iter().partition(|branch| {
        branch
            .worktree
            .as_deref()
            .is_some_and(|folder| !same_folder(folder, input.worktree))
    });
    let mut sentences = Vec::new();
    let mut result = UpdateRefs::default();
    let names: Vec<String> = here.iter().map(|branch| branch.name.clone()).collect();
    if !names.is_empty() {
        if !can {
            sentences.push(too_old(input.version, &names));
            result.staying.extend(names);
        } else if wanted {
            sentences.push(format!("These branches move with it: {}.", quoted(&names).join(", ")));
            result.moving = names;
        } else {
            sentences.push(format!("{} stay on the old commits.", and_list(&quoted(&names))));
            result.staying.extend(names);
        }
    }
    for branch in elsewhere {
        let folder = branch
            .worktree
            .as_deref()
            .map_or_else(String::new, |folder| folder.display().to_string());
        sentences.push(format!(
            "`{}` is checked out in {folder} and stays on the old commits: rebase it there.",
            branch.name
        ));
        result.staying.push(branch.name.clone());
    }
    result.option = match (can, wanted, result.moving.is_empty()) {
        (true, true, false) => Some("--update-refs"),
        (true, false, _) if stacked.update_refs == Some(true) => Some("--no-update-refs"),
        _ => None,
    };
    result.note = Some(sentences.join(" "));
    result
}

/// Why a git older than 2.38 leaves the branches behind.
fn too_old(version: Option<&GitVersion>, names: &[String]) -> String {
    let which = version.map_or_else(|| String::from("This git"), |version| format!("Git {}", version.said));
    let (major, minor) = UPDATE_REFS_VERSION.version;
    format!(
        "{which} cannot move {} with the rewrite ({major}.{minor} or later can): they stay on the old commits.",
        and_list(&quoted(names))
    )
}

/// The names in backticks.
fn quoted(names: &[String]) -> Vec<String> {
    names.iter().map(|name| format!("`{name}`")).collect()
}

/// `a`, `a and b`, `a, b and c`.
fn and_list(words: &[String]) -> String {
    match words {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// Whether two folders are the same, as git and the system may each write them.
fn same_folder(one: &Path, other: &Path) -> bool {
    let plain = |path: &Path| {
        let text = path.to_string_lossy().replace('\\', "/");
        let text = text.trim_end_matches('/').to_owned();
        if cfg!(windows) { text.to_lowercase() } else { text }
    };
    plain(one) == plain(other)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
