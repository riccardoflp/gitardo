//! The labels of each commit in the graph, and what each says.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::CommitId;
use crate::graph::combine_upstream::combine_upstream;
use crate::state::{Branch, RepositoryState};

/// The settings of the graph that change its labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphSettings {
    /// `graph.combineUpstream`: a branch and its upstream of the same name share one label while at the same commit.
    pub combine_upstream: bool,
}

impl Default for GraphSettings {
    fn default() -> Self {
        GraphSettings { combine_upstream: true }
    }
}

/// A label of a commit in the graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// What it says: `HEAD`, `main`, `origin/main`, `v1.0`, `stash@{0}`.
    pub name: String,
    /// What it stands for.
    pub kind: LabelKind,
    /// Whether it is what is checked out, which is the label filled in.
    pub is_checked_out: bool,
}

/// What a label stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelKind {
    /// `HEAD`, when nothing is checked out but a commit.
    Head,
    /// A local branch.
    Branch {
        /// The folder of another worktree that has it checked out.
        worktree: Option<PathBuf>,
        /// Its upstream, by its short name, when the two share this label (`graph.combine-upstream`).
        upstream: Option<String>,
    },
    /// A remote branch, with the icon of a cloud.
    RemoteBranch,
    /// A tag, lightweight or annotated.
    Tag,
    /// A stash, `stash@{n}`.
    Stash {
        /// Its place in the list of stashes.
        index: usize,
    },
}

/// The labels of each commit, in their order on it: `HEAD` when detached; the local branches, the one checked out
/// first and filled; the remote branches but `<remote>/HEAD` and those a branch's label stands for; the tags, at the
/// commit they point at; and the stashes made there.
///
/// Feature: graph.labels
#[must_use]
pub fn labels(state: &RepositoryState, settings: &GraphSettings) -> HashMap<CommitId, Vec<Label>> {
    let mut labels: HashMap<CommitId, Vec<Label>> = HashMap::new();
    let mut add = |commit: &CommitId, label: Label| labels.entry(commit.clone()).or_default().push(label);
    if let (None, Some(commit)) = (&state.head.branch, &state.head.commit) {
        add(commit, label("HEAD", LabelKind::Head, true));
    }
    let mut combined = HashSet::new();
    let checked_out_first = state
        .branches
        .iter()
        .filter(|branch| branch.is_checked_out)
        .chain(state.branches.iter().filter(|branch| !branch.is_checked_out));
    for branch in checked_out_first {
        let upstream = settings
            .combine_upstream
            .then(|| combine_upstream(branch, &state.remote_branches))
            .flatten();
        if let Some(upstream) = &upstream {
            let _ = combined.insert(upstream.clone());
        }
        let kind = LabelKind::Branch {
            worktree: other_worktree(state, branch),
            upstream,
        };
        add(&branch.commit, label(&branch.name, kind, branch.is_checked_out));
    }
    for remote_branch in &state.remote_branches {
        if !remote_branch.name.ends_with("/HEAD") && !combined.contains(&remote_branch.name) {
            add(
                &remote_branch.commit,
                label(&remote_branch.name, LabelKind::RemoteBranch, false),
            );
        }
    }
    for tag in &state.tags {
        add(&tag.commit, label(&tag.name, LabelKind::Tag, false));
    }
    for stash in &state.stashes {
        let name = format!("stash@{{{}}}", stash.index);
        add(
            &stash.commit,
            label(&name, LabelKind::Stash { index: stash.index }, false),
        );
    }
    labels
}

/// The folder of another worktree that has a branch checked out.
fn other_worktree(state: &RepositoryState, branch: &Branch) -> Option<PathBuf> {
    let full_name = format!("refs/heads/{}", branch.name);
    state
        .worktrees
        .iter()
        .find(|worktree| !worktree.is_current && worktree.branch.as_deref() == Some(full_name.as_str()))
        .map(|worktree| worktree.folder.clone())
}

/// A label.
fn label(name: &str, kind: LabelKind, is_checked_out: bool) -> Label {
    Label {
        name: name.to_owned(),
        kind,
        is_checked_out,
    }
}

#[cfg(test)]
mod tests;
