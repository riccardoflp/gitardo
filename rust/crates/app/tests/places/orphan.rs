//! A branch with no history, started from the window as Quick search and the **…** menu do (`branch.create-orphan`):
//! what the working tree keeps, the header and the timeline before its first commit, its first commit, and Undo.

use app::store::{Message, State};
use model::contract::{Answer, AnswerKey, Answers, FeatureId, Target};
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::feature::FeatureIntent;

use super::showing;
use super::support::{OperationsDriver, Window, says};

/// A repository on `main` with one commit of `README.md`, and `notes.txt` untracked.
fn repository() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let written = std::fs::write(repository.path().join("notes.txt"), "kept\n");
    assert!(written.is_ok(), "the untracked file is written");
    repository
}

/// Runs an operation with its answers given, as the entries that preset them do, says yes to its plan, and waits for
/// it to end.
fn run(driver: &OperationsDriver, state: &mut State, (feature, answers): (&'static str, Answers)) {
    let run = FeatureIntent::RunWith(FeatureId(feature), Target::Head, answers);
    driver.intent(state, Intent::Feature(run));
    loop {
        match driver.next(state) {
            Message::FeatureConfirms(_) => driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureEnded { .. } => return,
            other => panic!("{feature} asks nothing more here, and said {other:?}"),
        }
    }
}

/// The name `site` and what the working tree keeps.
fn site(keeps: &str) -> Answers {
    Answers::new()
        .with(AnswerKey("name"), Answer::Text(String::from("site")))
        .with(AnswerKey("keeps"), Answer::Choice(keeps.to_owned()))
}

/// The lines of git the log has.
fn ran(state: &State) -> Vec<String> {
    state
        .log
        .entries()
        .map(|entry| entry.line.clone())
        .filter(|line| line.starts_with("git "))
        .collect()
}

#[test]
fn branch_create_orphan__start_with_no_files_leaves_the_untracked_files_and_the_header_says_no_commits_yet() {
    let repository = repository();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    run(&driver, &mut state, ("branch.create-orphan", site("no-files")));
    assert!(
        ran(&state).contains(&String::from("git switch --orphan site")),
        "{:?}",
        ran(&state)
    );
    assert!(!repository.path().join("README.md").exists(), "the tracked file left");
    assert!(
        repository.path().join("notes.txt").exists(),
        "the untracked file stayed"
    );
    let (_, tree) = Window::new(ThemeName::Dark, (1000, 520)).draw(&state, &[]);
    assert!(
        says(&tree, "site (no commits yet)"),
        "the header names the branch with no commit"
    );
    let main = repository.git(&["rev-parse", "main"]);
    let main = model::CommitId::parse(main.trim()).unwrap_or_else(|| panic!("main has a commit"));
    assert!(
        state.timeline.rows.iter().any(|row| row.id() == Some(&main)),
        "the timeline lists main as before"
    );
}

#[test]
fn branch_create_orphan__keep_the_files_staged_stages_every_file_as_added() {
    let repository = repository();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    run(&driver, &mut state, ("branch.create-orphan", site("keep-files")));
    assert!(
        ran(&state).contains(&String::from("git checkout --orphan site")),
        "{:?}",
        ran(&state)
    );
    assert_eq!(repository.git(&["diff", "--cached", "--name-status"]), "A\tREADME.md\n");
}

#[test]
fn branch_create_orphan__the_first_commit_after_has_no_parent_and_is_drawn_apart() {
    let mut repository = repository();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    run(&driver, &mut state, ("branch.create-orphan", site("no-files")));
    let _ = repository.commit("Start the site", &[("index.html", "<h1>Gitardo</h1>\n")]);
    let _effects = super::support::run(&mut state, Message::Intent(Intent::Refresh));
    assert_eq!(
        repository.git(&["rev-list", "--count", "site"]).trim(),
        "1",
        "the commit has no parent"
    );
    assert_eq!(
        repository
            .git(&["rev-list", "--parents", "-n", "1", "site"])
            .split(' ')
            .count(),
        1
    );
    let first = repository.git(&["rev-parse", "site"]);
    let first = model::CommitId::parse(first.trim()).unwrap_or_else(|| panic!("site has a commit"));
    assert!(
        state.timeline.rows.iter().any(|row| row.id() == Some(&first)),
        "the timeline lists it"
    );
    // No solid line leaves its dot downwards: nothing joins it to the history of `main` under it.
    let layout = &state.timeline.layout;
    let cell = &layout.cells[layout.shown_of[&first]];
    assert!(
        !cell
            .lines
            .iter()
            .any(|line| line.half == model::graph::Half::Lower && line.from == cell.lane && !line.is_dashed),
        "{cell:?}"
    );
}

#[test]
fn branch_create_orphan__undo_right_after_checks_out_main_again_with_its_files() {
    let repository = repository();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    run(&driver, &mut state, ("branch.create-orphan", site("no-files")));
    run(&driver, &mut state, ("undo.last", Answers::new()));
    assert!(
        ran(&state).contains(&String::from("git checkout main")),
        "{:?}",
        ran(&state)
    );
    assert_eq!(repository.git(&["branch", "--show-current"]).trim(), "main");
    assert!(
        repository.path().join("README.md").exists(),
        "the files of main are back"
    );
}
