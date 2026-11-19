#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::contract::{Answer, Answers, FeatureId, Outcome, Target};
use model::state::{Skipped, SkippedFile};
use model::worktree_setup::{Ignored, PathStatus, Tracked};

use super::super::stop_tracking::PATH_STATUS;
use super::{ASSUME_UNCHANGED, HOW, SKIP_WORKTREE, skip, unskip};
use crate::testing::{context, lines, plan, question, state};

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

fn skipped(path: &str, how: Skipped) -> SkippedFile {
    SkippedFile {
        path: path.to_owned(),
        how,
    }
}

fn on(path: &str, answers: &Answers) -> Outcome {
    skip(&context(state(), Target::File(path.to_owned())), answers)
}

#[test]
fn changes_skip__skip_it_in_the_working_tree_runs_update_index_skip_worktree() {
    let mut answers = read(Tracked::It, false);
    let asked = question(on("config/local.json", &answers));
    assert_eq!(asked.default, Some(Answer::Choice(String::from(SKIP_WORKTREE))));
    answers.insert(HOW, Answer::Choice(String::from(SKIP_WORKTREE)));
    let plan = plan(on("config/local.json", &answers));
    assert_eq!(lines(&plan), ["git update-index --skip-worktree -- config/local.json"]);
}

#[test]
fn changes_skip__assume_it_is_unchanged_runs_update_index_assume_unchanged() {
    let mut answers = read(Tracked::It, false);
    answers.insert(HOW, Answer::Choice(String::from(ASSUME_UNCHANGED)));
    let plan = plan(on("config/local.json", &answers));
    assert_eq!(
        lines(&plan),
        ["git update-index --assume-unchanged -- config/local.json"]
    );
}

#[test]
fn changes_skip__a_name_with_brackets_is_given_as_it_is() {
    let mut answers = read(Tracked::It, false);
    answers.insert(HOW, Answer::Choice(String::from(SKIP_WORKTREE)));
    let plan = plan(on("pages/[id].tsx", &answers));
    assert_eq!(lines(&plan), ["git update-index --skip-worktree -- pages/[id].tsx"]);
}

#[test]
fn changes_skip__an_untracked_file_or_a_folder_is_refused_and_ignore_is_offered_for_the_first() {
    let Outcome::Refuse(refusal) = on("new.txt", &read(Tracked::Nothing, false)) else {
        panic!("a refusal");
    };
    assert_eq!(
        refusal.message.as_str(),
        "Git does not track new.txt: ignore it instead."
    );
    assert_eq!(
        refusal.offer.map(|(feature, _)| feature),
        Some(FeatureId("changes.ignore"))
    );
    assert!(matches!(
        on("config", &read(Tracked::FilesIn, true)),
        Outcome::Refuse(_)
    ));
}

#[test]
fn changes_unskip__a_file_skipped_is_shown_to_git_again() {
    let mut state = state();
    state.skipped.push(skipped("config/local.json", Skipped::Worktree));
    let plan = plan(unskip(
        &context(state, Target::File(String::from("config/local.json"))),
        &Answers::new(),
    ));
    assert_eq!(
        lines(&plan),
        ["git update-index --no-skip-worktree -- config/local.json"]
    );
}

#[test]
fn changes_unskip__a_file_hidden_both_ways_runs_both_commands() {
    let mut state = state();
    state.skipped.push(skipped("a.txt", Skipped::Both));
    let plan = plan(unskip(
        &context(state, Target::File(String::from("a.txt"))),
        &Answers::new(),
    ));
    assert_eq!(
        lines(&plan),
        [
            "git update-index --no-skip-worktree -- a.txt",
            "git update-index --no-assume-unchanged -- a.txt"
        ]
    );
}

#[test]
fn changes_unskip__show_all_to_git_again_runs_one_command_for_each_way() {
    let mut state = state();
    state.skipped.extend([
        skipped("a.txt", Skipped::Worktree),
        skipped("b.txt", Skipped::AssumedUnchanged),
        skipped("c.txt", Skipped::Worktree),
    ]);
    let plan = plan(unskip(&context(state, Target::Changes), &Answers::new()));
    assert_eq!(
        lines(&plan),
        [
            "git update-index --no-skip-worktree -- a.txt c.txt",
            "git update-index --no-assume-unchanged -- b.txt"
        ]
    );
}

#[test]
fn changes_stop_tracking__a_file_hidden_with_skip_worktree_is_shown_to_git_first() {
    let mut state = state();
    state.skipped.push(skipped("config/local.json", Skipped::Worktree));
    let plan = plan(super::super::stop_tracking(
        &context(state, Target::File(String::from("config/local.json"))),
        &read(Tracked::It, false),
    ));
    assert_eq!(
        lines(&plan),
        [
            "git update-index --no-skip-worktree -- config/local.json",
            "git rm --cached -- config/local.json"
        ]
    );
}
