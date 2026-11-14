//! Patches: commits copied or saved as one patch for `git am`, and a patch applied from a file or from the clipboard
//! (`docs/features/14-patches.md`).

mod apply;
mod copy;
mod save;

pub use apply::{CLIPBOARD, CONTENTS, FILE, FROM, TEMPORARY, WAY, apply};
pub use copy::copy;
pub use save::{PATH, save};

use model::CommitId;
use model::contract::{AnswerKey, Answers, Context, Outcome, Query, Refusal, Target};

/// The key of the patch git made of the commits chosen.
pub const PATCH: AnswerKey = AnswerKey("patch");

/// The commits chosen and the patch git made of them, oldest first, once it is read.
pub(crate) struct Made<'a> {
    /// The text of the patch.
    pub patch: &'a str,
    /// The commits of the patch, in the order of the history: the oldest first.
    pub commits: Vec<CommitId>,
}

impl Made<'_> {
    /// `<short hash>` for one commit, `N commits` for several.
    pub(crate) fn named(&self) -> String {
        match self.commits.as_slice() {
            [commit] => commit.short().to_owned(),
            commits => format!("{} commits", commits.len()),
        }
    }
}

/// The commits a feature was invoked on: one commit, or several selected.
fn chosen(context: &Context) -> Option<&[CommitId]> {
    match &context.target {
        Target::Commit(commit) => Some(std::slice::from_ref(commit)),
        Target::Commits(commits) if !commits.is_empty() => Some(commits),
        _ => None,
    }
}

/// The patch of the commits chosen: read first, then checked to have every commit; a merge has none of its own, and
/// is refused.
pub(crate) fn made<'a>(context: &Context, answers: &'a Answers) -> Result<Made<'a>, Box<Outcome>> {
    let Some(chosen) = chosen(context) else {
        return Err(Box::new(Outcome::Refuse(Refusal::saying("Pick a commit first."))));
    };
    let Some(patch) = answers.patch(PATCH) else {
        return Err(Box::new(Outcome::Read(
            PATCH,
            Query::FormatPatch {
                commits: chosen.to_vec(),
            },
        )));
    };
    let commits = commits_of(patch);
    if text::patch::count_commits(patch) < chosen.len() || commits.len() < chosen.len() {
        let message = if chosen.len() == 1 {
            "Git makes no patch of a merge commit."
        } else {
            "Git makes no patch of a merge commit: leave the merges out of the selection."
        };
        return Err(Box::new(Outcome::Refuse(Refusal::saying(message))));
    }
    Ok(Made { patch, commits })
}

/// The commits of a patch made by `git format-patch`, in its order: the hash after `From ` at the start of each.
fn commits_of(patch: &str) -> Vec<CommitId> {
    patch
        .lines()
        .filter_map(|line| line.strip_prefix("From "))
        .filter_map(|rest| rest.split(' ').next())
        .filter_map(CommitId::parse)
        .collect()
}

#[cfg(test)]
pub(crate) mod testing {
    use model::CommitId;

    /// A patch of `git format-patch` of some commits, in that order, each changing a file of its own.
    pub(crate) fn format_patch(commits: &[&CommitId]) -> String {
        let mut patch = String::new();
        for commit in commits {
            patch.push_str("From ");
            patch.push_str(commit.as_str());
            patch.push_str(" Mon Sep 17 00:00:00 2001\nFrom: Ada <ada@example.com>\nSubject: [PATCH] Add a\n\n---\n");
            patch.push_str(" a.txt | 1 +\n\ndiff --git a/a.txt b/a.txt\n--- a/a.txt\n+++ b/a.txt\n@@ -0,0 +1 @@\n+a\n");
        }
        patch
    }
}
