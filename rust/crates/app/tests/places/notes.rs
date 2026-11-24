//! The notes of commits in the window: the block **Note** under the message in the inspector and in the page of a
//! commit (`note.show`), and a note written or removed, then taken back by **Undo** (`note.edit`, `note.remove`).

use app::store::State;
use model::CommitId;
use model::contract::{Answer, AnswerKey, Answers, FeatureId, Target};
use render::{Rect, ThemeName};
use testkit::Repository;
use views::Intent;
use views::details::{DetailsIntent, NoteLength, NoteState};
use views::feature::FeatureIntent;

use super::inspector::{TALL, ask, drawn, select};
use super::showing;
use super::support::{OperationsDriver, Window, says};

/// Runs an operation of the catalog with its answers given, says yes to its plan when it shows one, and waits for it
/// to end.
fn run_to_the_end(
    driver: &OperationsDriver,
    state: &mut State,
    (feature, target, answers): (&'static str, Target, Answers),
) {
    driver.intent(
        state,
        Intent::Feature(FeatureIntent::RunWith(FeatureId(feature), target, answers)),
    );
    loop {
        match driver.next(state) {
            app::store::Message::FeatureConfirms(_) => {
                driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true)));
            }
            app::store::Message::FeatureEnded { .. } => return,
            other => panic!("{feature} asks nothing here, and said {other:?}"),
        }
    }
}

/// The note of a commit as `git notes show` prints it, or nothing.
fn note_of(repository: &Repository, commit: &str) -> String {
    repository
        .command("git")
        .args(["notes", "show", commit])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default()
}

/// The answer of the question of `note.edit`.
fn typed(note: &str) -> Answers {
    Answers::new().with(AnswerKey("note"), Answer::Text(note.to_owned()))
}

/// A repository of two commits, the last one with a note.
fn noted(note: &str) -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let _ = repository.commit("Add the parser", &[("parser.rs", "fn parse() {}\n")]);
    let _ = repository.git(&["notes", "add", "-m", note, "HEAD"]);
    repository
}

#[test]
fn note_show__a_commit_with_the_note_reviewed_shows_note_and_it_under_its_message_in_the_inspector_and_its_page() {
    let repository = noted("Reviewed.");
    let mut state = showing(repository.path().to_path_buf());
    let commit = select(&mut state, "Add the parser");
    let tree = drawn(&state);
    assert!(says(&tree, "Note"), "the inspector shows the block");
    assert!(says(&tree, "Reviewed."), "and the note");
    ask(&mut state, Intent::Details(DetailsIntent::OpenPage(commit)));
    let tree = drawn(&state);
    assert!(says(&tree, "Note"), "the page shows the block");
    assert!(says(&tree, "Reviewed."), "and the note");
}

#[test]
fn note_show__a_commit_with_no_note_shows_no_note() {
    let repository = noted("Reviewed.");
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Add the readme");
    let tree = drawn(&state);
    assert!(!says(&tree, "Note"));
    assert!(!says(&tree, "Reviewed."));
}

#[test]
fn note_show__a_note_of_three_lines_keeps_its_three_lines() {
    let commit = CommitId::parse(&"a".repeat(40)).unwrap();
    let block = |text: &'static str| NoteState {
        commit: &commit,
        text,
        length: NoteLength::Whole,
    };
    let mut window = Window::new(ThemeName::Dark, TALL);
    let ((one, three), drawn_tree) = window.frame(&[], |ui| {
        let heights = (
            views::details::note_height(ui, &block("Reviewed."), 400.0),
            views::details::note_height(ui, &block("Reviewed.\nTested on Windows.\nShipped."), 400.0),
        );
        let _ = ui.clip(Rect::new(0.0, 0.0, 400.0, 200.0), |ui| {
            views::details::note_view(&block("Reviewed.\nTested on Windows.\nShipped."), ui)
        });
        heights
    });
    let line = three - one;
    assert!(
        line > 0.0 && (line / 2.0 - 18.0).abs() < 8.0,
        "two lines more: {one} and {three}"
    );
    assert!(
        says(&drawn_tree, "Reviewed.\nTested on Windows.\nShipped."),
        "the lines are not joined"
    );
}

#[test]
fn note_show__the_block_matches_its_snapshots_in_each_theme() {
    let repository = noted("Reviewed.\nTested on Windows and on macOS.");
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Add the parser");
    for (theme, name) in [(ThemeName::Dark, "dark"), (ThemeName::Light, "light")] {
        let mut window = Window::new(theme, (1000, 520));
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(&format!("inspector-note-{name}"));
    }
}

#[test]
fn note_edit__undo_right_after_it_gives_the_commit_its_note_of_before_or_none() {
    let mut repository = Repository::new();
    let _ = repository.commit(
        "Add the readme",
        &[(
            "README.md",
            "Gitardo
",
        )],
    );
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let commit = select(&mut state, "Add the readme");
    let hash = commit.as_str().to_owned();
    let on_commit = || Target::Commit(commit.clone());
    run_to_the_end(&driver, &mut state, ("note.edit", on_commit(), typed("Reviewed.")));
    assert_eq!(
        note_of(&repository, &hash),
        "Reviewed.
"
    );
    run_to_the_end(
        &driver,
        &mut state,
        ("note.edit", on_commit(), typed("Reviewed and tested.")),
    );
    assert_eq!(
        note_of(&repository, &hash),
        "Reviewed and tested.
"
    );
    run_to_the_end(&driver, &mut state, ("undo.last", Target::Head, Answers::new()));
    assert_eq!(
        note_of(&repository, &hash),
        "Reviewed.
",
        "the note of before"
    );
    run_to_the_end(&driver, &mut state, ("undo.last", Target::Head, Answers::new()));
    assert_eq!(note_of(&repository, &hash), "", "none, as before the first note");
    assert_eq!(repository.git(&["for-each-ref", "refs/notes"]), "");
}

#[test]
fn note_remove__the_inspector_shows_no_note_after_it() {
    let repository = noted("Reviewed.");
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let commit = select(&mut state, "Add the parser");
    assert!(says(&drawn(&state), "Reviewed."));
    run_to_the_end(
        &driver,
        &mut state,
        ("note.remove", Target::Commit(commit.clone()), Answers::new()),
    );
    let log: Vec<String> = state
        .log
        .entries()
        .filter(|entry| super::support::is_ran(entry))
        .map(|entry| entry.line.clone())
        .collect();
    assert!(log.contains(&format!("git notes remove {commit}")), "{log:?}");
    let tree = drawn(&state);
    assert!(!says(&tree, "Note"));
    assert!(!says(&tree, "Reviewed."));
}

#[test]
fn note_remove__undo_right_after_it_gives_the_note_back() {
    let repository = noted("Reviewed.");
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let commit = select(&mut state, "Add the parser");
    run_to_the_end(
        &driver,
        &mut state,
        ("note.remove", Target::Commit(commit.clone()), Answers::new()),
    );
    assert_eq!(note_of(&repository, commit.as_str()), "");
    run_to_the_end(&driver, &mut state, ("undo.last", Target::Head, Answers::new()));
    assert_eq!(
        note_of(&repository, commit.as_str()),
        "Reviewed.
"
    );
}
