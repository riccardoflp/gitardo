#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::contract::{Answer, Answers, Danger, FollowUp, Outcome, Query, Step, Target};
use model::worktree_setup::{Ignored, PathStatus, Tracked};

use super::{PATH_STATUS, STAGED_DIFFERENT, stop_tracking};
use crate::testing::{context, lines, plan, state};

fn read(tracked: Tracked, is_folder: bool) -> Answers {
    let mut answers = Answers::new();
    answers.insert(
        PATH_STATUS,
        Answer::PathStatuses(vec![PathStatus {
            is_folder,
            tracked,
            ignored: Ignored::Nothing,
        }]),
    );
    answers
}

fn on(path: &str, answers: &Answers) -> Outcome {
    stop_tracking(&context(state(), Target::File(path.to_owned())), answers)
}

#[test]
fn changes_stop_tracking__a_file_is_asked_with_the_warning_then_taken_out_with_git_rm_cached() {
    assert!(matches!(
        on("config/local.json", &Answers::new()),
        Outcome::Read(_, Query::PathStatuses { .. })
    ));
    let plan = plan(on("config/local.json", &read(Tracked::It, false)));
    assert_eq!(lines(&plan), ["git rm --cached -- config/local.json"]);
    let Danger::Confirm(warning) = &plan.danger else {
        panic!("a confirmation: {:?}", plan.danger);
    };
    assert!(warning.as_str().starts_with("config/local.json stays on your disk."));
}

#[test]
fn changes_stop_tracking__a_folder_is_taken_out_with_r() {
    let plan = plan(on("generated", &read(Tracked::FilesIn, true)));
    assert_eq!(lines(&plan), ["git rm --cached -r -- generated"]);
}

#[test]
fn changes_stop_tracking__gits_refusal_is_followed_by_stop_tracking_anyway_shown_as_destructive() {
    let plan = plan(on("a.txt", &read(Tracked::It, false)));
    let [FollowUp::OfferWhenFailed(offer)] = plan.on_failure.as_slice() else {
        panic!("one offer: {:?}", plan.on_failure);
    };
    assert_eq!(offer.when, STAGED_DIFFERENT);
    assert_eq!(offer.plan.title.as_str(), "Stop tracking anyway");
    let forced: Vec<String> = offer.plan.steps.iter().map(Step::log_line).collect();
    assert_eq!(forced, ["git rm --cached --force -- a.txt"]);
    assert!(matches!(offer.plan.danger, Danger::Destructive(_)));
}

#[test]
fn changes_stop_tracking__a_file_git_does_not_track_is_refused() {
    let Outcome::Refuse(refusal) = on("new.txt", &read(Tracked::Nothing, false)) else {
        panic!("a refusal");
    };
    assert_eq!(refusal.message.as_str(), "Git does not track new.txt.");
}

#[test]
fn changes_stop_tracking__a_file_in_conflict_is_refused() {
    let mut state = state();
    state.working_tree.conflicted.push(model::state::ConflictedFile {
        path: String::from("a.txt"),
        kind: model::state::ConflictKind::BothModified,
    });
    let Outcome::Refuse(refusal) = stop_tracking(&context(state, Target::File(String::from("a.txt"))), &Answers::new())
    else {
        panic!("a refusal");
    };
    assert_eq!(refusal.message.as_str(), "a.txt is in conflict: resolve it first.");
}
