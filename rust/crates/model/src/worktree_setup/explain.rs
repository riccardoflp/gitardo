//! What becomes of a file or a folder given to new worktrees, or taken from them, from the Files page, as git works the
//! patterns out, and the sentence that says it (`worktree-setup.explain-change`).

use super::{PatternTake, SetupPatterns, SetupPreview, own_path_pattern, path_patterns};

/// What is asked from the Files page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupAction {
    /// **Link into New Worktrees**.
    Link,
    /// **Copy into New Worktrees**.
    Copy,
    /// **Stop Linking into New Worktrees**.
    StopLinking,
    /// **Stop Copying into New Worktrees**.
    StopCopying,
}

/// Whether git tracks a file, or files in a folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tracked {
    /// It tracks none of it.
    Nothing,
    /// It tracks the file.
    It,
    /// It tracks files in the folder.
    FilesIn,
}

/// How much of a file or a folder git ignores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ignored {
    /// All of it.
    Whole,
    /// Only part of the folder.
    Part,
    /// None of it.
    Nothing,
}

/// What [`explain_change`] looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExplainInput<'a> {
    /// The path from the top of the checkout, with `/`.
    pub path: &'a str,
    /// What is asked.
    pub action: SetupAction,
    /// Whether git tracks it, or files in it.
    pub tracked: Tracked,
    /// How much of it git ignores.
    pub ignored: Ignored,
    /// What the patterns take now.
    pub before: &'a SetupPreview,
    /// What they would take with the change.
    pub after: &'a SetupPreview,
}

/// What a choice from the Files page does: whether it writes the settings, and the sentence that says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupChange {
    /// Whether the settings are written: only when the change gives it something, or takes something away.
    pub writes: bool,
    /// What becomes of it.
    pub sentence: String,
}

/// What becomes of a file or a folder given to new worktrees or taken from them: given already, given by the change,
/// taken by a link before a copy, tracked by git, not ignored; or, when taken away, still given by another pattern.
///
/// Feature: worktree-setup.explain-change
#[must_use]
pub fn explain_change(input: &ExplainInput<'_>) -> SetupChange {
    match input.action {
        SetupAction::Link => give(input, "linked"),
        SetupAction::Copy => give(input, "copied"),
        SetupAction::StopLinking => stop(input, &input.after.linked, &input.after.link_patterns, "linked"),
        SetupAction::StopCopying => stop(input, &input.after.copied, &input.after.copy_patterns, "copied"),
    }
}

/// A change that writes nothing.
fn unchanged(sentence: String) -> SetupChange {
    SetupChange {
        writes: false,
        sentence,
    }
}

/// The patterns once a choice from the Files page is made on a path: to link or to copy, the pattern of its path goes
/// in that list and the patterns of its path come out of the other; to stop, the patterns of its path come out of
/// that list.
#[must_use]
pub fn change_patterns(
    patterns: &SetupPatterns,
    (path, is_folder): (&str, bool),
    action: SetupAction,
) -> SetupPatterns {
    let own = path_patterns(path, is_folder);
    let mut changed = patterns.clone();
    let (into, out_of) = match action {
        SetupAction::Link => (Some(&mut changed.link), &mut changed.copy),
        SetupAction::Copy => (Some(&mut changed.copy), &mut changed.link),
        SetupAction::StopLinking => (None, &mut changed.link),
        SetupAction::StopCopying => (None, &mut changed.copy),
    };
    out_of.retain(|pattern| !own.contains(pattern));
    if let Some(into) = into {
        let pattern = own_path_pattern(path, is_folder);
        if !into.contains(&pattern) {
            into.push(pattern);
        }
    }
    changed
}

/// Link or copy.
fn give(input: &ExplainInput<'_>, verb: &str) -> SetupChange {
    let name = input.path;
    let is_link = input.action == SetupAction::Link;
    let (before, before_patterns, after) = if is_link {
        (&input.before.linked, &input.before.link_patterns, &input.after.linked)
    } else {
        (&input.before.copied, &input.before.copy_patterns, &input.after.copied)
    };
    if takes(before, name) {
        let pattern = first_pattern(before_patterns, name);
        return unchanged(format!(
            "{name} is {verb} into new worktrees already, by the pattern {pattern}."
        ));
    }
    if !is_link
        && let Some(folder) = input
            .before
            .linked
            .iter()
            .find(|folder| is_same_or_inside(name, folder))
    {
        let pattern = first_pattern(&input.before.link_patterns, name);
        return unchanged(if folder == name {
            format!("{name} is linked into new worktrees by the pattern {pattern}, which comes before copying.")
        } else {
            format!(
                "{name} is in {folder}, which the pattern {pattern} links into new worktrees: nothing in it is copied."
            )
        });
    }
    if takes(after, name) {
        return SetupChange {
            writes: true,
            sentence: format!("{name} will be {verb} into new worktrees."),
        };
    }
    if !is_link && after.iter().any(|path| is_same_or_inside(path, name)) {
        return SetupChange {
            writes: true,
            sentence: format!("What git ignores in {name} will be copied into new worktrees."),
        };
    }
    let rest = "only what it ignores is linked or copied into new worktrees.";
    unchanged(match (input.tracked, input.ignored) {
        (Tracked::It, _) => format!("Git tracks {name}: each worktree has it from its commit."),
        (Tracked::FilesIn, _) if is_link => {
            format!("Git tracks files in {name}: each worktree has them from its commit.")
        }
        (_, Ignored::Part) => format!("Git ignores only part of {name}: {rest}"),
        _ => format!("Git does not ignore {name}: {rest}"),
    })
}

/// Stop linking or copying.
fn stop(input: &ExplainInput<'_>, after: &[String], after_patterns: &[PatternTake], verb: &str) -> SetupChange {
    let name = input.path;
    let sentence = if takes(after, name) {
        let pattern = first_pattern(after_patterns, name);
        format!("{name} is still {verb} into new worktrees, by the pattern {pattern}.")
    } else {
        format!("New worktrees will no longer get {name}.")
    };
    SetupChange { writes: true, sentence }
}

/// Whether paths taken take a path as a whole: it, or a folder it is in.
fn takes(taken: &[String], path: &str) -> bool {
    taken.iter().any(|taken| is_same_or_inside(path, taken))
}

/// The first pattern that by itself takes a path or a folder it is in.
fn first_pattern(patterns: &[PatternTake], path: &str) -> String {
    patterns
        .iter()
        .find(|take| takes(&take.paths, path))
        .map_or_default(|take| take.pattern.clone())
}

/// Whether a path is a folder, or inside it.
fn is_same_or_inside(path: &str, folder: &str) -> bool {
    path == folder || path.strip_prefix(folder).is_some_and(|rest| rest.starts_with('/'))
}
