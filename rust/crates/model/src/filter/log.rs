//! Which history the timeline lists: the arguments of `git log` that the branches shown give.

use std::ffi::OsString;

use super::{BranchesShown, ReferenceKind};
use crate::CommitId;

/// The arguments of `git log` that say which commits the timeline lists and in which order: `--date-order`, then the
/// branches shown. `HEAD` is left out before the first commit; `refs/stash` is never listed. References ticked one by
/// one follow `--ignore-missing`, so that one deleted since it was chosen is passed over, and only names under
/// `refs/heads/`, `refs/remotes/` and `refs/tags/` are given, so that a name is never taken for an option.
///
/// Feature: filter.log-arguments
#[must_use]
pub fn log_arguments(shown: Option<&BranchesShown>, head: Option<&CommitId>, upstream: Option<&str>) -> Vec<OsString> {
    let mut arguments = vec![OsString::from("--date-order")];
    let head = head.map(|_| OsString::from("HEAD"));
    match shown {
        None => {
            arguments.extend(["--branches", "--remotes", "--tags"].map(OsString::from));
            arguments.extend(head);
        }
        Some(BranchesShown::Kinds(kinds)) => {
            let flags = [
                (kinds.local, "--branches"),
                (kinds.remote, "--remotes"),
                (kinds.tags, "--tags"),
            ];
            arguments.extend(
                flags
                    .into_iter()
                    .filter(|(is_shown, _)| *is_shown)
                    .map(|(_, flag)| OsString::from(flag)),
            );
            arguments.extend(head);
        }
        Some(BranchesShown::Current) => arguments.extend(head),
        Some(BranchesShown::Upstream) => {
            arguments.extend(head);
            let upstream = upstream.filter(|name| name.starts_with("refs/"));
            arguments.extend(upstream.map(OsString::from));
        }
        Some(BranchesShown::References(names)) => {
            arguments.push(OsString::from("--ignore-missing"));
            let known = names.iter().filter(|name| ReferenceKind::of(name).is_some());
            arguments.extend(known.map(OsString::from));
        }
    }
    arguments
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::ffi::OsString;

    use super::super::{BranchesShown, Kinds};
    use super::log_arguments;
    use crate::graph::testing::id;

    fn words(arguments: &[OsString]) -> String {
        arguments
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn filter_log_arguments__everything_shown_lists_every_branch_remote_branch_tag_and_head() {
        let head = id("A");
        assert_eq!(
            words(&log_arguments(None, Some(&head), None)),
            "--date-order --branches --remotes --tags HEAD"
        );
    }

    #[test]
    fn filter_log_arguments__local_branches_and_tags() {
        let head = id("A");
        let shown = BranchesShown::Kinds(Kinds {
            local: true,
            remote: false,
            tags: true,
        });
        assert_eq!(
            words(&log_arguments(Some(&shown), Some(&head), None)),
            "--date-order --branches --tags HEAD"
        );
    }

    #[test]
    fn filter_log_arguments__upstream_on_main_tracking_origin_main() {
        let head = id("A");
        assert_eq!(
            words(&log_arguments(
                Some(&BranchesShown::Upstream),
                Some(&head),
                Some("refs/remotes/origin/main")
            )),
            "--date-order HEAD refs/remotes/origin/main"
        );
    }

    #[test]
    fn filter_log_arguments__main_and_v1_0_ticked() {
        let head = id("A");
        let shown = BranchesShown::References(vec![String::from("refs/heads/main"), String::from("refs/tags/v1.0")]);
        assert_eq!(
            words(&log_arguments(Some(&shown), Some(&head), None)),
            "--date-order --ignore-missing refs/heads/main refs/tags/v1.0"
        );
    }

    #[test]
    fn filter_log_arguments__in_a_repository_with_no_commits_head_is_left_out() {
        assert_eq!(
            words(&log_arguments(None, None, None)),
            "--date-order --branches --remotes --tags"
        );
    }

    #[test]
    fn filter_log_arguments__a_ticked_name_that_is_not_a_reference_is_never_given() {
        let shown = BranchesShown::References(vec![String::from("--all"), String::from("refs/stash")]);
        assert_eq!(
            words(&log_arguments(Some(&shown), None, None)),
            "--date-order --ignore-missing"
        );
    }
}
