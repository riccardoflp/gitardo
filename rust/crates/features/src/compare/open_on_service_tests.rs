use model::compare::{CompareMode, CompareSide, ComparisonPart, ComparisonTarget, Side};
use model::contract::{Answer, Answers, FollowUp, Outcome, Query, Target};
use model::remote::Remote;
use model::state::RepositoryState;

use super::{REMOTES, open_on_service};
use crate::compare::testing::{branch as side, compared};
use crate::testing::{branch, commit, context, plan, state, tracking};

/// `main` at `a` and `feature` at `b`, each tracking the branch of `origin` of its name.
fn repository() -> RepositoryState {
    let mut repository = state();
    let tracked = |name: &str, digit: char| model::state::Branch {
        upstream: Some(tracking("origin", name)),
        ..branch(name, digit, name == "main")
    };
    repository.branches = vec![tracked("main", 'a'), tracked("feature", 'b')];
    repository
}

/// The remotes: `origin` at a URL.
fn origin(url: &str) -> Answers {
    let origin = Remote {
        name: String::from("origin"),
        url: url.to_owned(),
    };
    Answers::new().with(REMOTES, Answer::Remotes(vec![origin]))
}

/// What opening a comparison with `origin` at a URL gives.
fn outcome(target: ComparisonTarget, url: &str) -> Outcome {
    let context = context(repository(), Target::Comparison(Box::new(target)));
    open_on_service(&context, &origin(url))
}

/// The URL opened for a comparison with `origin` at a URL.
fn opened(target: ComparisonTarget, url: &str) -> String {
    match plan(outcome(target, url)).after.as_slice() {
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

/// A comparison of two sides, everything that differs.
fn everything(first: CompareSide, second: CompareSide) -> ComparisonTarget {
    ComparisonTarget {
        mode: CompareMode::Everything,
        ..compared(first, second, ComparisonPart::Whole)
    }
}

const GITLAB: &str = "git@gitlab.com:group/app.git";

#[test]
fn compare_open_on_service__main_and_feature_on_gitlab_open_its_comparison_and_straight_for_everything() {
    let target = since_diverged(side("main", 'a'), side("feature", 'b'));
    let context = context(repository(), Target::Comparison(Box::new(target.clone())));
    assert_eq!(
        open_on_service(&context, &Answers::new()),
        Outcome::Read(REMOTES, Query::Remotes)
    );
    assert!(
        plan(outcome(target.clone(), GITLAB)).steps.is_empty(),
        "nothing changes"
    );
    assert_eq!(
        opened(target, GITLAB),
        "https://gitlab.com/group/app/-/compare/main...feature"
    );
    assert_eq!(
        opened(everything(side("main", 'a'), side("feature", 'b')), GITLAB),
        "https://gitlab.com/group/app/-/compare/main...feature?straight=true"
    );
}

#[test]
fn compare_open_on_service__on_azure_devops_a_side_given_by_its_hash_is_named_gc_and_its_hash() {
    let fixed = CompareSide {
        name: commit('e').short().to_owned(),
        side: Side::Commit(commit('e')),
        is_followed: false,
        is_gone: false,
    };
    let url = opened(
        since_diverged(side("main", 'a'), fixed),
        "https://org@dev.azure.com/org/Project/_git/app",
    );
    assert_eq!(
        url,
        format!(
            "https://dev.azure.com/org/Project/_git/app/branchCompare?baseVersion=GBmain&targetVersion=GC{}",
            commit('e')
        )
    );
}

#[test]
fn compare_open_on_service__on_bitbucket_it_is_refused_in_everything_that_differs() {
    let bitbucket = "https://bitbucket.org/team/app.git";
    let refused = outcome(everything(side("main", 'a'), side("feature", 'b')), bitbucket);
    assert!(matches!(refused, Outcome::Refuse(_)), "{refused:?}");
    assert_eq!(
        opened(since_diverged(side("main", 'a'), side("feature", 'b')), bitbucket),
        "https://bitbucket.org/team/app/branches/compare/feature%0Dmain"
    );
}

#[test]
fn compare_open_on_service__on_github_it_opens_what_compare_open_on_github_opens() {
    let url = opened(
        since_diverged(side("main", 'a'), side("feature", 'b')),
        "https://github.com/owner/app.git",
    );
    assert_eq!(url, "https://github.com/owner/app/compare/main...feature");
    let refused = |target: ComparisonTarget, url: &str| matches!(outcome(target, url), Outcome::Refuse(_));
    assert!(refused(
        since_diverged(side("main", 'a'), CompareSide::working_tree()),
        GITLAB
    ));
    assert!(refused(since_diverged(side("main", 'a'), side("main", 'a')), GITLAB));
    assert!(refused(
        since_diverged(side("main", 'a'), side("feature", 'b')),
        "git@example.com:owner/app.git"
    ));
}
