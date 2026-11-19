//! What a rewrite changed, from the window: **See what changed** after a rebase and after a fetch that found an upstream
//! force pushed, and **Compare first** in the warning of a force push, each opening the page of the pairs
//! (`range-diff.compare`, `range-diff.page`).

use app::store::range_diff::PairsRead;
use app::store::{Message, State};
use model::contract::{FeatureId, FollowUp, Target};
use model::range_diff::PairKind;
use testkit::{Remote, Repository};
use views::Intent;
use views::feature::FeatureIntent;

use super::showing;
use super::support::OperationsDriver;

/// `feature` two commits above an older `main`, `main` one commit further, `feature` checked out.
fn diverged() -> Repository {
    let mut repository = Repository::new();
    let _a = repository.commit("Add a", &[("a.txt", "a\n")]);
    repository.branch("feature");
    repository.checkout("feature");
    let _b = repository.commit("Add b", &[("b.txt", "b\n")]);
    let _c = repository.commit("Add c", &[("c.txt", "c\n")]);
    repository.checkout("main");
    let _d = repository.commit("Add d", &[("d.txt", "d\n")]);
    repository.checkout("feature");
    repository
}

/// Runs an operation as a click does, says yes to its plan, and waits until it ended; the plans it showed.
fn run_to_the_end(driver: &OperationsDriver, state: &mut State, (feature, target): (FeatureId, Target)) -> usize {
    driver.intent(state, Intent::Feature(FeatureIntent::Run(feature, target)));
    let mut shown = 0;
    loop {
        match driver.next(state) {
            Message::FeatureConfirms(_) => {
                shown += 1;
                driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true)));
            }
            Message::FeatureEnded { .. } => return shown,
            _ => {}
        }
    }
}

/// The kinds of the pairs of the page, once read.
fn pairs_shown(state: &State) -> Vec<PairKind> {
    let Some(page) = &state.range_diff else {
        panic!("the page is open")
    };
    match &page.read {
        PairsRead::Read(read) => read.pairs.iter().map(|pair| pair.kind).collect(),
        other => panic!("the pairs are read: {other:?}"),
    }
}

/// The operations what is said offers, each with the words of its button.
fn offered(state: &State) -> Vec<(String, FeatureId, Target)> {
    let Some(said) = &state.operations.said else {
        panic!("something is said")
    };
    said.runs
        .iter()
        .map(|run| (run.label.as_str().to_owned(), run.feature, run.target.clone()))
        .collect()
}

#[test]
fn range_diff_compare__right_after_rebase_feature_onto_main_what_is_said_has_see_what_changed_which_opens_the_page() {
    let repository = diverged();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let _rebased = run_to_the_end(
        &driver,
        &mut state,
        (FeatureId("rebase.onto"), Target::Branch(String::from("main"))),
    );
    let offers = offered(&state);
    let Some((_, feature, target)) = offers.iter().find(|(label, ..)| label == "See what changed") else {
        panic!("offers See what changed: {offers:?}")
    };
    let _opened = run_to_the_end(&driver, &mut state, (*feature, target.clone()));
    assert_eq!(pairs_shown(&state), [PairKind::Same, PairKind::Same]);
    let page = state.range_diff.as_ref().expect("the page is open");
    assert_eq!(page.target.title, "What the rebase of feature changed");
    assert_eq!(page.title, "feature \u{b7} what changed");
}

#[test]
fn range_diff_compare__before_a_force_push_of_a_branch_rebased_after_it_was_pushed_compare_first_opens_the_page_and_pushes_nothing()
 {
    let repository = diverged();
    let remote = Remote::new(&repository);
    let _pushed = repository.git(&["push", "--quiet", "--set-upstream", "origin", "feature"]);
    let _rebased = repository.git(&["rebase", "--quiet", "main"]);
    let on_the_remote = remote.git(&["rev-parse", "feature"]);
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    driver.intent(
        &mut state,
        Intent::Feature(FeatureIntent::Run(
            FeatureId("sync.push-force-with-lease"),
            Target::Head,
        )),
    );
    let mut ended = 0;
    while ended < 2 {
        match driver.next(&mut state) {
            Message::FeatureConfirms(plan) => {
                let instead = plan.after.iter().find_map(|follow_up| match follow_up {
                    FollowUp::OfferBeforeRun(offered) => Some(offered.clone()),
                    _ => None,
                });
                let Some(instead) = instead else {
                    panic!("the warning offers Compare first: {plan:?}")
                };
                assert_eq!(instead.label.as_str(), "Compare first");
                driver.intent(
                    &mut state,
                    Intent::Feature(FeatureIntent::RunInstead(
                        instead.feature,
                        instead.target,
                        instead.answers,
                    )),
                );
            }
            Message::FeatureEnded { .. } => ended += 1,
            _ => {}
        }
    }
    assert_eq!(
        remote.git(&["rev-parse", "feature"]),
        on_the_remote,
        "nothing is pushed"
    );
    let page = state.range_diff.as_ref().expect("the page is open");
    assert_eq!(page.target.title, "feature and origin/feature, commit by commit");
    assert_eq!(pairs_shown(&state), [PairKind::Same, PairKind::Same]);
}

#[test]
fn range_diff_compare__after_a_fetch_that_found_origin_feature_force_pushed_while_feature_follows_it_it_offers_see_what_changed()
 {
    let mut repository = Repository::new();
    let _a = repository.commit("Add a", &[("a.txt", "a\n")]);
    let remote = Remote::new(&repository);
    repository.branch("feature");
    repository.checkout("feature");
    let _b = repository.commit("Add b", &[("b.txt", "b\n")]);
    let _pushed = repository.git(&["push", "--quiet", "--set-upstream", "origin", "feature"]);
    repository.checkout("main");
    repository.branch("other");
    repository.checkout("other");
    let other = repository.commit("Add c", &[("c.txt", "c\n")]);
    let _other_pushed = repository.git(&["push", "--quiet", "origin", "other"]);
    repository.checkout("feature");
    // Someone else force pushes `feature` to another commit.
    let _forced = remote.git(&["update-ref", "refs/heads/feature", &other]);
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let _fetched = run_to_the_end(&driver, &mut state, (FeatureId("sync.fetch"), Target::Head));
    let said = state.operations.said.as_ref().expect("something is said");
    assert_eq!(said.text, "`origin/feature` was force pushed.");
    let offers = offered(&state);
    let Some((_, feature, target)) = offers.iter().find(|(label, ..)| label == "See what changed") else {
        panic!("offers See what changed: {offers:?}")
    };
    assert_eq!(*target, Target::RemoteBranch(String::from("origin/feature")));
    let _opened = run_to_the_end(&driver, &mut state, (*feature, target.clone()));
    let page = state.range_diff.as_ref().expect("the page is open");
    assert_eq!(page.target.title, "What the force push of origin/feature changed");
    assert_eq!(pairs_shown(&state), [PairKind::Dropped, PairKind::Added]);
}
