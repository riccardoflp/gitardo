use model::Timestamp;
use model::contract::{Answer, AnswerKey, Answers, FollowUp, Outcome, Query, Target};
use model::range_diff::{RangeDiffTarget, Rewrite, RewriteKind};
use model::state::RemoteBranch;

use super::{WITH, WITH_UPSTREAM, compare, compare_first, see_what_changed};
use crate::testing::{branch, commit, context, plan, state, tracking};

/// The key the last rewrite is read under.
const LAST_REWRITE: AnswerKey = AnswerKey("range-diff.last-rewrite");

/// A rebase of `feature` from `1…` to `2…` onto `3…`.
fn rebase() -> Rewrite {
    Rewrite {
        kind: RewriteKind::Rebase { onto: commit('3') },
        before: commit('1'),
        after: commit('2'),
    }
}

/// A repository on `main`, with `feature` at `2…` following `origin/feature` at `1…`.
fn repository() -> model::state::RepositoryState {
    let mut state = state();
    let mut feature = branch("feature", '2', false);
    feature.upstream = Some(tracking("origin", "feature"));
    state.branches.push(feature);
    state.remote_branches.push(RemoteBranch {
        name: String::from("origin/feature"),
        remote: String::from("origin"),
        commit: commit('1'),
        date: Timestamp(0),
    });
    state
}

/// The answers with the last rewrite read.
fn read(rewrite: Option<Rewrite>) -> Answers {
    Answers::new().with(LAST_REWRITE, Answer::LastRewrite(rewrite))
}

/// What the plan of an outcome opens.
fn opened(outcome: Outcome) -> RangeDiffTarget {
    let plan = plan(outcome);
    assert!(plan.steps.is_empty(), "it changes nothing");
    match plan.after.as_slice() {
        [FollowUp::OpenRangeDiff(target)] => (**target).clone(),
        other => panic!("opens the page: {other:?}"),
    }
}

#[test]
fn range_diff_compare__first_reads_the_last_rewrite_of_the_branch() {
    let outcome = compare(
        &context(repository(), Target::Branch(String::from("feature"))),
        &Answers::new(),
    );
    assert_eq!(
        outcome,
        Outcome::Read(
            LAST_REWRITE,
            Query::LastRewrite {
                reference: String::from("refs/heads/feature")
            }
        )
    );
}

#[test]
fn range_diff_compare__what_its_last_rewrite_changed_on_a_branch_rebased_in_a_terminal_opens_the_page() {
    let target = opened(compare(
        &context(repository(), Target::Branch(String::from("feature"))),
        &read(Some(rebase())),
    ));
    assert_eq!(target.title, "What the rebase of feature changed");
    assert_eq!(target.branch, "feature");
    assert_eq!(
        target.ranges.arguments,
        [commit('3'), commit('1'), commit('2')].map(|commit| commit.to_string())
    );
}

#[test]
fn range_diff_compare__on_a_branch_never_rebased_or_amended_it_says_so_and_opens_nothing() {
    let outcome = compare(
        &context(repository(), Target::Branch(String::from("feature"))),
        &read(None),
    );
    let Outcome::Refuse(refusal) = outcome else {
        panic!("refuses: {outcome:?}")
    };
    assert_eq!(
        refusal.message.as_str(),
        "feature has not been rebased or amended lately."
    );
}

#[test]
fn range_diff_compare__compare_first_before_a_force_push_opens_the_upstream_against_the_branch() {
    let FollowUp::OfferBeforeRun(offered) = compare_first("feature") else {
        panic!("a button of the warning")
    };
    assert_eq!(offered.label.as_str(), "Compare first");
    let mut on_feature = repository();
    on_feature.remotes.push(String::from("origin"));
    on_feature.branches[0].is_checked_out = false;
    on_feature.branches[1].is_checked_out = true;
    on_feature.head.branch = Some(String::from("refs/heads/feature"));
    let lost = model::remote::Overwritten {
        commits: vec![model::remote::OverwrittenCommit {
            short_id: String::from("1111111"),
            subject: String::from("Add the parser"),
        }],
        has_more: false,
    };
    let overwritten = Answers::new().with(AnswerKey("overwritten"), Answer::Overwritten(lost));
    let push = plan(crate::sync::push_force(
        &context(on_feature, Target::Head),
        &overwritten,
    ));
    assert!(push.after.contains(&compare_first("feature")), "{:?}", push.after);
    let target = opened(compare(
        &context(repository(), offered.target.clone()),
        &offered
            .answers
            .clone()
            .with(LAST_REWRITE, Answer::LastRewrite(Some(rebase()))),
    ));
    assert_eq!(target.title, "feature and origin/feature, commit by commit");
    assert_eq!(
        target.ranges.arguments,
        [
            commit('3').to_string(),
            String::from("origin/feature"),
            String::from("feature")
        ],
        "rebased after it was pushed: the ranges of the rebase, from the upstream"
    );
    let same = Answers::new()
        .with(WITH, Answer::Choice(WITH_UPSTREAM.to_owned()))
        .with(LAST_REWRITE, Answer::LastRewrite(None));
    let mut at_the_same_commit = repository();
    at_the_same_commit.remote_branches[0].commit = commit('2');
    let outcome = compare(
        &context(at_the_same_commit, Target::Branch(String::from("feature"))),
        &same,
    );
    let Outcome::Refuse(refusal) = outcome else {
        panic!("refuses: {outcome:?}")
    };
    assert_eq!(
        refusal.message.as_str(),
        "feature and origin/feature are at the same commit."
    );
}

#[test]
fn range_diff_compare__after_a_fetch_that_found_origin_feature_force_pushed_it_compares_it_before_and_after() {
    let pushed = Rewrite {
        kind: RewriteKind::ForcePush,
        ..rebase()
    };
    let target = opened(compare(
        &context(repository(), Target::RemoteBranch(String::from("origin/feature"))),
        &read(Some(pushed)),
    ));
    assert_eq!(target.title, "What the force push of origin/feature changed");
    assert_eq!(target.ranges.arguments, [format!("{}...{}", commit('1'), commit('2'))]);
}

#[test]
fn range_diff_compare__right_after_rebase_feature_onto_main_what_is_said_has_see_what_changed() {
    let FollowUp::OfferRun(offered) = see_what_changed("feature") else {
        panic!("a button of what is said")
    };
    assert_eq!(offered.label.as_str(), "See what changed");
    assert_eq!(offered.target, Target::Branch(String::from("feature")));
    let target = opened(compare(
        &context(repository(), offered.target.clone()),
        &read(Some(rebase())),
    ));
    assert_eq!(target.title, "What the rebase of feature changed");
}
