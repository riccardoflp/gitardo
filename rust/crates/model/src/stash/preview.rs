//! What applying a stash would stop on, said before it is applied or popped.

use super::StashFiles;

/// How many files a list of the note names before it says how many more there are.
const FILES_SHOWN: usize = 15;

/// What the note of `stash.apply` and `stash.pop` is worked out from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StashPreviewInput<'a> {
    /// The files the stash changes, and the untracked files it took.
    pub files: &'a StashFiles,
    /// The files of the working tree that have changes, staged or not, untracked ones left out.
    pub changed_here: &'a [String],
    /// The untracked files of the stash that exist in the working tree.
    pub untracked_there: &'a [String],
    /// The files `git merge-tree` names in conflict, or none when git cannot tell.
    pub conflicts: Option<&'a [String]>,
}

/// The paragraphs of the note of the plan, after the note of `stash.apply` or `stash.pop`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StashPreview {
    /// Each paragraph, its list of files one per line after its first line.
    pub paragraphs: Vec<String>,
}

/// What git would stop on if the stash were applied now: the changed files it would refuse to touch, the untracked
/// files it took that are there again, and the conflicts with what was committed since; or **No conflicts expected.**
/// A stash git would refuse says nothing of conflicts, as git would not get that far.
///
/// Feature: stash.preview
#[must_use]
pub fn preview(input: &StashPreviewInput<'_>) -> StashPreview {
    let mut paragraphs = Vec::new();
    let refused: Vec<&String> = input
        .files
        .changed
        .iter()
        .filter(|file| input.changed_here.contains(file))
        .collect();
    if !refused.is_empty() {
        let head = if refused.len() == 1 {
            String::from("1 file it changes has")
        } else {
            format!("{} files it changes have", refused.len())
        };
        paragraphs.push(listed(
            &format!(
                "Git will refuse it and change nothing, as {head} changes that are not committed: commit or stash them \
                 first."
            ),
            &refused,
        ));
    }
    if !input.untracked_there.is_empty() {
        let head = match input.untracked_there.len() {
            1 => String::from(
                "An untracked file it took along is there again: git leaves it as it is and keeps the stash.",
            ),
            count => format!(
                "{count} untracked files it took along are there again: git leaves them as it is and keeps the stash."
            ),
        };
        let files: Vec<&String> = input.untracked_there.iter().collect();
        paragraphs.push(listed(&head, &files));
    }
    if refused.is_empty() {
        match input.conflicts {
            Some([]) => paragraphs.push(String::from("No conflicts expected.")),
            Some(conflicts) => {
                let count = if conflicts.len() == 1 {
                    String::from("1 file")
                } else {
                    format!("{} files", conflicts.len())
                };
                let files: Vec<&String> = conflicts.iter().collect();
                paragraphs.push(listed(&format!("This will stop with conflicts in {count}:"), &files));
            }
            None => {} // Git cannot tell: nothing is said of the conflicts.
        }
    }
    StashPreview { paragraphs }
}

/// A sentence followed by the first files of a list, one per line, and **…and K more** for the rest.
fn listed(head: &str, files: &[&String]) -> String {
    let mut lines = vec![head.to_owned()];
    lines.extend(files.iter().take(FILES_SHOWN).map(|file| (*file).clone()));
    if files.len() > FILES_SHOWN {
        lines.push(format!("\u{2026}and {} more", files.len() - FILES_SHOWN));
    }
    lines.join("\n")
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{StashPreviewInput, preview};
    use crate::stash::StashFiles;

    fn owned(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| (*name).to_owned()).collect()
    }

    fn files(changed: &[&str], untracked: &[&str]) -> StashFiles {
        StashFiles {
            changed: owned(changed),
            untracked: owned(untracked),
        }
    }

    #[test]
    fn stash_preview__a_clean_apply_says_no_conflicts_expected() {
        let files = files(&["a.txt"], &[]);
        let input = StashPreviewInput {
            files: &files,
            changed_here: &[],
            untracked_there: &[],
            conflicts: Some(&[]),
        };
        assert_eq!(preview(&input).paragraphs, ["No conflicts expected."]);
    }

    #[test]
    fn stash_preview__a_file_changed_here_is_named_and_conflicts_are_not_said() {
        let files = files(&["a.txt", "b.txt"], &[]);
        let here = owned(&["a.txt", "c.txt"]);
        let conflicts = owned(&["b.txt"]);
        let input = StashPreviewInput {
            files: &files,
            changed_here: &here,
            untracked_there: &[],
            conflicts: Some(&conflicts),
        };
        assert_eq!(
            preview(&input).paragraphs,
            [
                "Git will refuse it and change nothing, as 1 file it changes has changes that are not committed: commit \
              or stash them first.\na.txt"
            ]
        );
    }

    #[test]
    fn stash_preview__an_untracked_file_there_again_is_named() {
        let files = files(&[], &["new.txt", "other.txt"]);
        let there = owned(&["new.txt"]);
        let input = StashPreviewInput {
            files: &files,
            changed_here: &[],
            untracked_there: &there,
            conflicts: None,
        };
        assert_eq!(
            preview(&input).paragraphs,
            ["An untracked file it took along is there again: git leaves it as it is and keeps the stash.\nnew.txt"]
        );
    }

    #[test]
    fn stash_preview__a_conflict_with_a_later_commit_names_the_file() {
        let files = files(&["b.txt"], &[]);
        let conflicts = owned(&["b.txt"]);
        let input = StashPreviewInput {
            files: &files,
            changed_here: &[],
            untracked_there: &[],
            conflicts: Some(&conflicts),
        };
        assert_eq!(
            preview(&input).paragraphs,
            ["This will stop with conflicts in 1 file:\nb.txt"]
        );
    }

    #[test]
    fn stash_preview__twenty_conflicts_list_fifteen_and_five_more() {
        let conflicts: Vec<String> = (0..20).map(|index| format!("file{index:02}.txt")).collect();
        let files = StashFiles {
            changed: conflicts.clone(),
            untracked: Vec::new(),
        };
        let input = StashPreviewInput {
            files: &files,
            changed_here: &[],
            untracked_there: &[],
            conflicts: Some(&conflicts),
        };
        let paragraphs = preview(&input).paragraphs;
        let lines: Vec<&str> = paragraphs.iter().flat_map(|paragraph| paragraph.lines()).collect();
        assert_eq!(lines.first(), Some(&"This will stop with conflicts in 20 files:"));
        assert_eq!(lines.len(), 17);
        assert_eq!(lines.last(), Some(&"\u{2026}and 5 more"));
    }
}
