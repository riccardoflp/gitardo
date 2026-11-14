use model::compare::{CompareMode, CompareSide, ComparisonPart, ComparisonTarget, Side};
use model::contract::{Answer, Answers, FollowUp, Outcome, Query, Target};
use model::remote::Remote;
use model::state::{RemoteBranch, RepositoryState};

use super::{REMOTES, open_on_github};
use crate::compare::testing::{branch as side, compared};
use crate::testing::{branch, commit, context, plan, state, tag, tracking};

/// `main` at `a` and `feat-x` at `b`, each tracking the branch of `origin` of its name, `feature` at `b` tracking
/// `origin/feat-x`, the remote branches `origin/main` and `origin/release`, and the tag `v1.0`.
fn repository() -> RepositoryState {
    let mut repository = state();
    let tracked = |name: &str, digit: char, upstream: &str| model::state::Branch {
        upstream: Some(tracking("origin", upstream)),
        ..branch(name, digit, name == "main")
    };
    repository.branches = vec![
        tracked("main", 'a', "main"),
        tracked("feature", 'b', "feature"),
        tracked("local-x", 'b', "feat-x"),
    ];
    repository.remote_branches = ["main", "release"]
        .into_iter()
        .map(|name| RemoteBranch {
            name: format!("origin/{name}"),
            remote: String::from("origin"),
            commit: commit('c'),
            date: model::Timestamp(0),
        })
        .collect();
    repository.tags = vec![tag("v1.0", 'd')];
    repository
}

/// The remotes: `origin` on GitHub.
fn on_github() -> Answers {
    let origin = Remote {
        name: String::from("origin"),
        url: String::from("https://github.com/owner/app.git"),
    };
    Answers::new().with(REMOTES, Answer::Remotes(vec![origin]))
}

/// The URL opened for a comparison.
fn opened(target: ComparisonTarget) -> String {
    let context = context(repository(), Target::Comparison(Box::new(target)));
    let after = plan(open_on_github(&context, &on_github())).after;
    match after.as_slice() {
        [FollowUp::OpenUrl(url)] => url.clone(),
        other => panic!("opens one URL, not {other:?}"),
    }
}

/// A comparison of two sides since they diverged.
fn since_diverged(first: CompareSide, second: CompareSide) -> ComparisonTarget {
    ComparisonTarget {
        mode: CompareMode::SinceDiverged,
        ..compared(first, second, ComparisonPart::Whole)
    }
}

#[test]
fn compare_open_on_github__main_and_feature_tracking_origin_on_github_open_main_three_dots_feature() {
    let target = since_diverged(side("main", 'a'), side("feature", 'b'));
    let context = context(repository(), Target::Comparison(Box::new(target.clone())));
    assert_eq!(
        open_on_github(&context, &Answers::new()),
        Outcome::Read(REMOTES, Query::Remotes)
    );
    let plan = plan(open_on_github(&context, &on_github()));
    assert!(plan.steps.is_empty(), "nothing changes");
    assert_eq!(opened(target), "https://github.com/owner/app/compare/main...feature");
}

#[test]
fn compare_open_on_github__a_local_branch_whose_upstream_is_origin_feat_x_is_named_feat_x() {
    let url = opened(since_diverged(side("main", 'a'), side("local-x", 'b')));
    assert_eq!(url, "https://github.com/owner/app/compare/main...feat-x");
    let remote = side("origin/release", 'c');
    let tagged = side("v1.0", 'd');
    let url = opened(since_diverged(remote, tagged));
    assert_eq!(url, "https://github.com/owner/app/compare/release...v1.0");
}

#[test]
fn compare_open_on_github__a_side_given_by_its_hash_is_named_by_its_full_hash() {
    let fixed = CompareSide {
        name: commit('e').short().to_owned(),
        side: Side::Commit(commit('e')),
        is_followed: false,
        is_gone: false,
    };
    let gone = CompareSide {
        is_gone: true,
        ..side("deleted", 'f')
    };
    let url = opened(since_diverged(fixed, gone));
    let (fixed, gone) = (commit('e'), commit('f'));
    assert_eq!(url, format!("https://github.com/owner/app/compare/{fixed}...{gone}"));
}

#[test]
fn compare_open_on_github__in_everything_that_differs_the_url_has_two_dots() {
    let target = compared(side("main", 'a'), side("feature", 'b'), ComparisonPart::Whole);
    assert_eq!(target.mode, CompareMode::Everything);
    assert_eq!(opened(target), "https://github.com/owner/app/compare/main..feature");
}

#[test]
fn compare_open_on_github__refuses_the_working_tree_the_same_commit_and_another_host() {
    let refused = |target: ComparisonTarget, answers: &Answers| {
        let context = context(repository(), Target::Comparison(Box::new(target)));
        matches!(open_on_github(&context, answers), Outcome::Refuse(_))
    };
    let working_tree = since_diverged(side("main", 'a'), CompareSide::working_tree());
    assert!(refused(working_tree, &on_github()));
    assert!(refused(
        since_diverged(side("main", 'a'), side("v1.0", 'a')),
        &on_github()
    ));
    let elsewhere = Answers::new().with(
        REMOTES,
        Answer::Remotes(vec![Remote {
            name: String::from("origin"),
            url: String::from("https://gitlab.com/owner/app.git"),
        }]),
    );
    assert!(refused(
        since_diverged(side("main", 'a'), side("feature", 'b')),
        &elsewhere
    ));
}
