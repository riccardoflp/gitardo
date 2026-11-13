//! A patch of two commits applied with `git am -3` whose second commit does not fit (`patch.am-stopped`): the banner
//! with **Continue**, **Skip** and **Abort**, each run to the end, and the sides of the file in conflict named in the
//! Merge page.

use app::store::{Message, State};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::feature::FeatureIntent;
use views::merge_view::MergeIntent;

use super::inspector::ask;
use super::support::{OperationsDriver, Window, find, has, says};

/// The size of the window.
const WIDE: (u32, u32) = (1200, 560);

/// The subjects of the commits of `HEAD`, newest first.
fn subjects(repository: &Repository) -> Vec<String> {
    repository
        .git(&["log", "--format=%s"])
        .lines()
        .map(str::to_owned)
        .collect()
}

/// A patch of `feature`'s two commits, **Add the notes** and **Change the second line**, applied with `git am -3` onto
/// `main`, which changed the second line its own way: the first commit is made, and the second stops on `file.txt`.
fn stopped() -> Repository {
    let mut repository = Repository::new();
    let _base = repository.commit("Add the file", &[("file.txt", "1\n2\n3\n")]);
    repository.branch("feature");
    let _main = repository.commit("Write the second line in French", &[("file.txt", "1\ndeux\n3\n")]);
    repository.checkout("feature");
    let _notes = repository.commit("Add the notes", &[("notes.txt", "notes\n")]);
    let _second = repository.commit("Change the second line", &[("file.txt", "1\ntwo\n3\n")]);
    let patch = repository.git(&["format-patch", "--stdout", "main..feature"]);
    repository.checkout("main");
    let file = repository.path().join(".git").join("series.patch");
    std::fs::write(&file, patch).unwrap();
    let output = repository
        .command("git")
        .args(["am", "-3", file.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!output.status.success(), "the patch stops on its second commit");
    repository
}

/// Whether `git am` is still under way.
fn is_applying(repository: &Repository) -> bool {
    repository.path().join(".git").join("rebase-apply").exists()
}

/// Clicks a button of the banner, and runs what it starts to its end, saying yes to what it asks; gives the window and
/// whether it asked first.
fn press(repository: &Repository, button: &str) -> (State, bool) {
    let mut state = super::showing(repository.path().to_path_buf());
    let intents = Window::new(ThemeName::Dark, WIDE).click(&state, Role::Button, button);
    let driver = OperationsDriver::new();
    for intent in intents {
        driver.intent(&mut state, intent);
    }
    let mut is_asked = false;
    loop {
        match driver.next(&mut state) {
            Message::FeatureConfirms(_) | Message::FeatureAsks(_) => {
                is_asked = true;
                driver.intent(&mut state, Intent::Feature(FeatureIntent::Confirm(true)));
            }
            Message::FeatureEnded { .. } => return (state, is_asked),
            _ => {}
        }
    }
}

/// The git commands the operations ran, as the Command log has them.
fn logged(state: &State) -> Vec<String> {
    state.log.entries().map(|entry| entry.line.clone()).collect()
}

#[test]
fn patch_am_stopped__a_patch_whose_second_commit_conflicts_stops_with_the_first_made_and_the_banner_has_continue_skip_and_abort()
 {
    let repository = stopped();
    assert_eq!(subjects(&repository).first().map(String::as_str), Some("Add the notes"));
    let state = super::showing(repository.path().to_path_buf());
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    assert!(says(&tree, "Patch (git am) in progress \u{b7} 1 file has conflicts"));
    for button in ["Continue", "Skip", "Abort"] {
        assert!(has(&tree, Role::Button, button), "{button} is not drawn");
    }
    assert!(
        find(&tree, Role::Button, "Continue").is_disabled(),
        "a file is still in conflict"
    );
}

#[test]
fn patch_am_stopped__continue_after_the_file_is_resolved_makes_that_commit() {
    let repository = stopped();
    std::fs::write(repository.path().join("file.txt"), "1\ntwo\n3\n").unwrap();
    let _ = repository.git(&["add", "file.txt"]);
    let (state, _) = press(&repository, "Continue");
    assert_eq!(
        logged(&state).last().map(String::as_str),
        Some("git -c core.editor=true am --continue")
    );
    assert!(!is_applying(&repository));
    let subjects = subjects(&repository);
    assert_eq!(
        subjects.iter().take(3).map(String::as_str).collect::<Vec<_>>(),
        [
            "Change the second line",
            "Add the notes",
            "Write the second line in French"
        ]
    );
}

#[test]
fn patch_am_stopped__skip_leaves_that_commit_out_and_goes_on() {
    let repository = stopped();
    let (state, _) = press(&repository, "Skip");
    assert_eq!(logged(&state).last().map(String::as_str), Some("git am --skip"));
    assert!(!is_applying(&repository));
    assert_eq!(
        subjects(&repository)
            .iter()
            .take(2)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["Add the notes", "Write the second line in French"]
    );
    assert_eq!(
        std::fs::read_to_string(repository.path().join("file.txt")).unwrap(),
        "1\ndeux\n3\n"
    );
}

#[test]
fn patch_am_stopped__abort_asks_then_goes_back_to_where_the_branch_was_before_the_patch() {
    let repository = stopped();
    let before = repository.git(&["rev-parse", "HEAD~1"]);
    let (state, is_asked) = press(&repository, "Abort");
    assert!(is_asked, "Abort asks first");
    assert_eq!(logged(&state).last().map(String::as_str), Some("git am --abort"));
    assert!(!is_applying(&repository));
    assert_eq!(repository.git(&["rev-parse", "HEAD"]), before);
    assert_eq!(
        subjects(&repository).first().map(String::as_str),
        Some("Write the second line in French")
    );
}

#[test]
fn patch_am_stopped__the_merge_page_names_the_sides_the_branch_checked_out_and_the_patch_being_applied() {
    let repository = stopped();
    let mut state = super::showing(repository.path().to_path_buf());
    ask(
        &mut state,
        Intent::MergeView(MergeIntent::Open(String::from("file.txt"))),
    );
    let (_, tree) = Window::new(ThemeName::Dark, (1200, 700)).draw(&state, &[]);
    assert!(says(&tree, "Current \u{2014} The branch checked out"));
    assert!(says(&tree, "Incoming \u{2014} The patch being applied"));
}
