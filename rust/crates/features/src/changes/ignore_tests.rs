#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::PathBuf;

use model::contract::{Answer, AnswerKey, Answers, Danger, Outcome, QuestionKind, Target};
use model::worktree_setup::{Ignored, PathStatus, Tracked};

use super::{ONLY_WRITE, STOP_TRACKING, TRACKED, TYPED, TYPED_CHOICE, WHAT, WHERE, ignore};
use crate::testing::{context, lines, plan, question, state};

/// The answers of the reads, for a path that nothing ignores yet, with the text of the file the line goes in.
fn read(tracked: Tracked, is_folder: bool, text: Option<&str>) -> Answers {
    let mut answers = Answers::new();
    answers.insert(AnswerKey("ignore-ignored-by"), Answer::Names(Vec::new()));
    answers.insert(
        AnswerKey("stop-tracking-path-status"),
        Answer::PathStatuses(vec![PathStatus {
            is_folder,
            tracked,
            ignored: Ignored::Nothing,
        }]),
    );
    answers.insert(
        AnswerKey("ignore-exclude-file"),
        Answer::Path(PathBuf::from("repository/.git/info/exclude")),
    );
    answers.insert(
        AnswerKey("ignore-file-text"),
        Answer::Files(vec![text.map(|text| text.as_bytes().to_vec())]),
    );
    answers
}

fn on(path: &str, answers: &Answers) -> Outcome {
    ignore(&context(state(), Target::File(path.to_owned())), answers)
}

fn choose(answers: &mut Answers, key: AnswerKey, choice: &str) {
    answers.insert(key, Answer::Choice(choice.to_owned()));
}

#[test]
fn changes_ignore__all_log_files_in_the_repository_gitignore_writes_star_log_at_its_end() {
    let mut answers = read(Tracked::Nothing, false, Some("target/\n"));
    choose(&mut answers, WHAT, "1");
    choose(&mut answers, WHERE, "0");
    let plan = plan(on("debug.log", &answers));
    let [model::contract::Step::Write { path, contents }] = plan.steps.as_slice() else {
        panic!("one write: {:?}", plan.steps);
    };
    assert_eq!(path, &PathBuf::from("repository").join(".gitignore"));
    assert_eq!(contents, b"target/\n*.log\n");
    assert!(lines(&plan)[0].starts_with("write ") && lines(&plan)[0].ends_with(".gitignore"));
    assert_eq!(plan.danger, Danger::None);
}

#[test]
fn changes_ignore__only_on_this_computer_writes_the_exclude_file() {
    let mut answers = read(Tracked::Nothing, false, None);
    choose(&mut answers, WHAT, "0");
    let QuestionKind::Pick { choices } = question(on("debug.log", &answers)).kind else {
        panic!("a pick of the places");
    };
    let exclude = choices.last().unwrap();
    assert_eq!(exclude.label.as_str(), "Only on this computer");
    assert!(
        exclude
            .description
            .as_ref()
            .unwrap()
            .as_str()
            .contains("Not committed, and not shared with anyone")
    );
    choose(&mut answers, WHERE, &exclude.identifier);
    let plan = plan(on("debug.log", &answers));
    let [model::contract::Step::Write { path, contents }] = plan.steps.as_slice() else {
        panic!("one write: {:?}", plan.steps);
    };
    assert_eq!(path, &PathBuf::from("repository/.git/info/exclude"));
    assert_eq!(contents, b"/debug.log\n");
}

#[test]
fn changes_ignore__this_folder_on_build_writes_slash_build_slash() {
    let mut answers = read(Tracked::Nothing, true, Some(""));
    let asked = question(on("build", &answers));
    assert_eq!(asked.default, Some(Answer::Choice(String::from("0"))));
    choose(&mut answers, WHAT, "0");
    choose(&mut answers, WHERE, "0");
    let plan = plan(on("build", &answers));
    let model::contract::Step::Write { contents, .. } = &plan.steps[0] else {
        panic!("a write");
    };
    assert_eq!(contents, b"/build/\n");
}

#[test]
fn changes_ignore__on_a_tracked_file_stop_tracking_it_writes_then_runs_git_rm_cached() {
    let mut answers = read(Tracked::It, false, None);
    choose(&mut answers, WHAT, "0");
    choose(&mut answers, WHERE, "0");
    let asked = question(on("config/local.json", &answers));
    assert_eq!(asked.key, TRACKED);
    assert_eq!(asked.default, Some(Answer::Choice(String::from(STOP_TRACKING))));
    choose(&mut answers, TRACKED, STOP_TRACKING);
    let plan = plan(on("config/local.json", &answers));
    let lines = lines(&plan);
    assert!(lines[0].ends_with(".gitignore"), "{lines:?}");
    assert_eq!(lines[1], "git rm --cached -- config/local.json");
    assert!(matches!(plan.danger, Danger::Confirm(_)));
}

#[test]
fn changes_ignore__only_write_the_pattern_on_a_tracked_file_runs_no_git_rm() {
    let mut answers = read(Tracked::It, false, None);
    choose(&mut answers, WHAT, "0");
    choose(&mut answers, WHERE, "0");
    choose(&mut answers, TRACKED, ONLY_WRITE);
    let plan = plan(on("config/local.json", &answers));
    assert_eq!(plan.steps.len(), 1);
    assert!(!lines(&plan).iter().any(|line| line.contains("git rm")));
    assert_eq!(plan.danger, Danger::None);
}

#[test]
fn changes_ignore__a_pattern_typed_is_written_as_typed() {
    let mut answers = read(Tracked::Nothing, false, None);
    choose(&mut answers, WHAT, TYPED_CHOICE);
    assert_eq!(question(on("debug.log", &answers)).key, TYPED);
    answers.insert(TYPED, Answer::Text(String::from("tmp-*")));
    choose(&mut answers, WHERE, "0");
    let model::contract::Step::Write { contents, .. } = &plan(on("debug.log", &answers)).steps[0] else {
        panic!("a write");
    };
    assert_eq!(contents, b"tmp-*\n");
}

#[test]
fn changes_ignore__a_path_ignored_already_is_refused_with_its_pattern_and_its_file() {
    let mut answers = Answers::new();
    answers.insert(
        AnswerKey("ignore-ignored-by"),
        Answer::Names(vec![String::from(".gitignore"), String::from("*.log")]),
    );
    let Outcome::Refuse(refusal) = on("x.log", &answers) else {
        panic!("a refusal");
    };
    assert_eq!(
        refusal.message.as_str(),
        "x.log is ignored already, by *.log in .gitignore."
    );
}

#[test]
fn changes_ignore__backing_out_of_any_question_writes_nothing() {
    let mut answers = read(Tracked::It, false, None);
    assert!(matches!(on("a.txt", &answers), Outcome::Ask(_)));
    choose(&mut answers, WHAT, "0");
    assert!(matches!(on("a.txt", &answers), Outcome::Ask(_)));
    choose(&mut answers, WHERE, "0");
    assert!(matches!(on("a.txt", &answers), Outcome::Ask(_)));
}

#[test]
fn changes_ignore__a_path_in_a_submodule_is_refused() {
    let mut state = state();
    state.submodules.push(model::submodule::Submodule {
        name: String::from("lib"),
        folder: String::from("lib"),
        parent: None,
        url: None,
        branch: None,
        recorded: None,
        staged: None,
        checkout: None,
    });
    let Outcome::Refuse(refusal) = ignore(&context(state, Target::File(String::from("lib/a.o"))), &Answers::new())
    else {
        panic!("a refusal");
    };
    assert_eq!(
        refusal.message.as_str(),
        "lib/a.o is in the submodule lib: ignore it there."
    );
}
