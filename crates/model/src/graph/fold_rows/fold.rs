//! The row that stands for the commits a merge brought in, and what it says.

use std::collections::HashMap;

use crate::CommitId;
use crate::commit::Commit;
use crate::graph::labels::{Label, LabelKind};

/// The commits a folded merge brought in, put away under one row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fold {
    /// The merge.
    pub merge: CommitId,
    /// The commits put away, from the top.
    pub commits: Vec<CommitId>,
    /// The commit the row stands for in the lines of the graph: the first of them.
    pub id: CommitId,
    /// The commits they come from that are listed, in the order of the rows.
    pub parents: Vec<CommitId>,
    /// The merge's second parent, whose branch the commits are of.
    pub second_parent: CommitId,
    /// The branch the merge's message names: `x` of `Merge branch 'x'`, `Merge remote-tracking branch 'x'` or
    /// `Merge pull request #n from x`.
    pub merged_name: Option<String>,
}

impl Fold {
    /// The fold of the commits a merge brought in, from the top, and the commits they come from.
    pub(super) fn new(merge: &Commit, commits: Vec<CommitId>, parents: Vec<CommitId>) -> Fold {
        Fold {
            merge: merge.id.clone(),
            id: commits.first().cloned().unwrap_or_else(|| merge.id.clone()),
            commits,
            parents,
            second_parent: merge.parents.get(1).cloned().unwrap_or_else(|| merge.id.clone()),
            merged_name: merged_name(&merge.subject),
        }
    }

    /// The branch the commits are of: the branch or remote branch that labels the merge's second parent, or else the
    /// name in the message of the merge.
    #[must_use]
    pub fn branch(&self, labels: &HashMap<CommitId, Vec<Label>>) -> Option<String> {
        let at_second_parent = labels.get(&self.second_parent).map_or(&[][..], Vec::as_slice);
        let named = |wanted: fn(&LabelKind) -> bool| {
            at_second_parent
                .iter()
                .find(|label| wanted(&label.kind))
                .map(|label| label.name.clone())
        };
        named(|kind| matches!(kind, LabelKind::Branch { .. }))
            .or_else(|| named(|kind| matches!(kind, LabelKind::RemoteBranch)))
            .or_else(|| self.merged_name.clone())
    }

    /// What the row says: `3 commits of feature`, or `1 commit merged` when the branch is not known.
    #[must_use]
    pub fn title(&self, labels: &HashMap<CommitId, Vec<Label>>) -> String {
        let count = match self.commits.len() {
            1 => String::from("1 commit"),
            count => format!("{count} commits"),
        };
        match self.branch(labels) {
            Some(branch) => format!("{count} of {branch}"),
            None => format!("{count} merged"),
        }
    }
}

/// The branch the message of a merge names, as git writes it: `Merge branch 'x'`, `Merge remote-tracking branch 'x'`,
/// or as GitHub does, `Merge pull request #n from x`.
fn merged_name(subject: &str) -> Option<String> {
    let quoted = ["Merge branch '", "Merge remote-tracking branch '"]
        .iter()
        .find_map(|prefix| subject.strip_prefix(prefix))
        .and_then(|rest| rest.split_once('\''))
        .map(|(name, _)| name);
    let pull_request = || {
        subject
            .strip_prefix("Merge pull request #")
            .and_then(|rest| rest.split_once(" from "))
            .and_then(|(_, rest)| rest.split_whitespace().next())
    };
    quoted
        .or_else(pull_request)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::merged_name;

    #[test]
    fn reads_the_branch_the_message_of_a_merge_names() {
        let cases = [
            ("Merge branch 'feature'", Some("feature")),
            ("Merge branch 'feature' into main", Some("feature")),
            ("Merge remote-tracking branch 'origin/fix'", Some("origin/fix")),
            ("Merge pull request #7 from octo/fix", Some("octo/fix")),
            ("Merge pull request #7 from octo/fix\tand more", Some("octo/fix")),
            ("Add the parser", None),
            ("Merge branch ''", None),
        ];
        for (subject, expected) in cases {
            assert_eq!(merged_name(subject).as_deref(), expected, "{subject}");
        }
    }
}
