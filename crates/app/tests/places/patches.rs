//! Patches from the window: a patch of commits applied and taken back by Undo at once, and the save dialog cancelled.

use app::store::{Effect, Message, State};
use model::CommitId;
use model::contract::{Answer, AnswerKey, Answers, FeatureId, QuestionKind, Target};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::feature::FeatureIntent;

use super::showing;
use super::support::{OperationsDriver, Window, has, run};

/// `main` with the commits `Add a` to `Add d`, their hashes in that order.
fn four_commits() -> (Repository, Vec<String>) {
    let mut repository = Repository::new();
    let hashes = ["a", "b", "c", "d"]
        .iter()
        .map(|name| repository.commit(&format!("Add {name}"), &[(&format!("{name}.txt"), "line\n")]))
        .map(|hash| hash.trim().to_owned())
        .collect();
    (repository, hashes)
}

/// Waits until the operation ends, confirming its plan.
fn until_ended(driver: &OperationsDriver, state: &mut State) {
    loop {
        match driver.next(state) {
            Message::FeatureConfirms(_) => driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureEnded { .. } => return,
            _ => {}
        }
    }
}

#[test]
fn patch_apply__undo_right_after_a_patch_of_three_commits_takes_back_the_three_at_once() {
    let (repository, hashes) = four_commits();
    let first = hashes.first().unwrap().clone();
    let patch = repository.git(&["format-patch", "--stdout", &format!("{first}..HEAD")]);
    let file = repository.path().join(".git").join("three.patch");
    std::fs::write(&file, patch).unwrap();
    let _back = repository.git(&["reset", "--quiet", "--hard", &first]);
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let answers = Answers::new()
        .with(AnswerKey("from"), Answer::Choice(String::from("file")))
        .with(AnswerKey("file"), Answer::Path(file))
        .with(AnswerKey("as"), Answer::Choice(String::from("commits")));
    let apply = FeatureIntent::RunWith(FeatureId("patch.apply"), Target::Head, answers);
    driver.intent(&mut state, Intent::Feature(apply));
    until_ended(&driver, &mut state);
    assert_eq!(repository.git(&["rev-list", "--count", "HEAD"]).trim(), "4");
    let mut window = Window::new(ThemeName::Dark, (1000, 520));
    assert!(has(&window.draw(&state, &[]).1, Role::Button, "Undo"));
    driver.intent(
        &mut state,
        Intent::Feature(FeatureIntent::Run(FeatureId("undo.last"), Target::Head)),
    );
    until_ended(&driver, &mut state);
    assert_eq!(repository.git(&["rev-parse", "HEAD"]).trim(), first);
    let undone: Vec<String> = state.log.entries().map(|entry| entry.line.clone()).collect();
    assert_eq!(undone.last(), Some(&format!("git reset --keep {first}")));
}

#[test]
fn patch_save__cancelling_the_dialog_writes_nothing() {
    let (repository, hashes) = four_commits();
    let commit = CommitId::parse(hashes.last().unwrap()).unwrap();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let save = FeatureIntent::Run(FeatureId("patch.save"), Target::Commit(commit));
    driver.intent(&mut state, Intent::Feature(save));
    let asked = loop {
        if let Message::FeatureAsks(question) = driver.next(&mut state) {
            break question;
        }
    };
    assert!(matches!(asked.kind, QuestionKind::SaveFile(_)), "{asked:?}");
    let effects = run(
        &mut state,
        Message::Intent(Intent::Feature(FeatureIntent::Answer(None))),
    );
    driver.take(&state, &effects);
    until_ended(&driver, &mut state);
    assert!(!effects.iter().any(|effect| matches!(effect, Effect::ChooseFile { .. })));
    assert_eq!(state.log.entries().count(), 0, "nothing ran");
    assert_eq!(repository.git(&["status", "--porcelain"]), "");
}
