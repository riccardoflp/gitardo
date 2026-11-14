//! A branch and its upstream on one label, while they are at the same commit.

use crate::state::{Branch, RemoteBranch};

/// The upstream a branch shares its label with, by its short name (`origin/feature`): when the branch has an upstream
/// that is not gone, with the same name on its remote as the branch here, and that remote branch is at the same commit
/// as the branch. None otherwise: each has its own label.
///
/// Feature: graph.combine-upstream
#[must_use]
pub fn combine_upstream(branch: &Branch, remote_branches: &[RemoteBranch]) -> Option<String> {
    let upstream = branch.upstream.as_ref().filter(|upstream| !upstream.is_gone)?;
    let same_name = upstream
        .remote_branch
        .strip_prefix("refs/heads/")
        .is_some_and(|name| name == branch.name);
    if !same_name {
        return None;
    }
    let name = upstream.reference.strip_prefix("refs/remotes/")?;
    remote_branches
        .iter()
        .find(|remote_branch| remote_branch.name == name && remote_branch.commit == branch.commit)
        .map(|remote_branch| remote_branch.name.clone())
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::combine_upstream;
    use crate::graph::testing::{branch, id, remote_branch};

    #[test]
    fn graph_combine_upstream__a_branch_and_its_upstream_at_the_same_commit_share_a_label() {
        let feature = branch("feature", "A", Some(("origin", "feature")));
        let remote = [remote_branch("origin/feature", "A")];
        assert_eq!(combine_upstream(&feature, &remote).as_deref(), Some("origin/feature"));
    }

    #[test]
    fn graph_combine_upstream__a_commit_on_the_branch_gives_each_its_label() {
        let mut feature = branch("feature", "A", Some(("origin", "feature")));
        feature.commit = id("B");
        let remote = [remote_branch("origin/feature", "A")];
        assert_eq!(combine_upstream(&feature, &remote), None);
    }

    #[test]
    fn graph_combine_upstream__an_upstream_of_another_name_has_its_own_label() {
        let feature = branch("feature", "A", Some(("origin", "other")));
        let remote = [remote_branch("origin/other", "A")];
        assert_eq!(combine_upstream(&feature, &remote), None);
    }

    #[test]
    fn graph_combine_upstream__an_upstream_gone_or_missing_has_no_label_to_share() {
        let mut feature = branch("feature", "A", Some(("origin", "feature")));
        assert_eq!(combine_upstream(&feature, &[]), None);
        if let Some(upstream) = feature.upstream.as_mut() {
            upstream.is_gone = true;
        }
        assert_eq!(
            combine_upstream(&feature, &[remote_branch("origin/feature", "A")]),
            None
        );
    }
}
