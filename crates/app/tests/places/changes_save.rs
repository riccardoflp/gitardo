//! **Save** and **Save all** on the Changes page (`changes.save`): the edits typed in File pages and not saved yet
//! written to disk, so that git sees them, and the list then showing the files as git sees them; and which files with
//! edits not saved the list has (`changes.unsaved`).

use app::store::State;
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use ui::editor::Command;
use views::Intent;
use views::changes::ChangesIntent;
use views::file_view::{FilePageIntent, FileTarget, FileViewIntent};

use super::support::{OperationsDriver, Window, find, has, says};

/// The size of the window.
const WIDE: (u32, u32) = (1000, 520);

/// A repository with one commit of `a.txt` and `b.txt`.
fn committed() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the files", &[("a.txt", "a\n"), ("b.txt", "b\n")]);
    repository
}

/// A window showing a repository, with a File page for each file given, its text replaced and not saved, and the
/// Changes page open.
fn edited(repository: &Repository, files: &[(&str, &str)]) -> (State, OperationsDriver) {
    let mut state = super::showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    for (path, text) in files {
        driver.intent(
            &mut state,
            Intent::FileView(FileViewIntent::Open(FileTarget::working_tree(path))),
        );
        let id = state
            .file_pages
            .pages
            .iter()
            .find(|page| page.path == *path)
            .unwrap()
            .id;
        let commands = vec![Command::SelectAll, Command::Paste((*text).to_owned())];
        driver.intent(
            &mut state,
            Intent::FileView(FileViewIntent::Page(id, FilePageIntent::Edit(commands))),
        );
    }
    driver.intent(&mut state, Intent::Changes(ChangesIntent::Open));
    (state, driver)
}

/// Waits for the operation running to end, confirming what it asks.
fn until_ended(driver: &OperationsDriver, state: &mut State) {
    super::compare::until_ended(driver, state);
}

/// The git commands and the writes the operations ran, as the Command log has them.
fn logged(state: &State) -> Vec<String> {
    state.log.entries().map(|entry| entry.line.clone()).collect()
}

/// The file on disk.
fn on_disk(repository: &Repository, path: &str) -> String {
    std::fs::read_to_string(repository.path().join(path)).unwrap()
}

/// Whether a page has edits not saved.
fn is_unsaved(state: &State, path: &str) -> bool {
    state
        .file_pages
        .pages
        .iter()
        .any(|page| page.path == path && page.is_unsaved())
}

/// Draws the window.
fn drawn(state: &State) -> TreeUpdate {
    Window::new(ThemeName::Dark, WIDE).draw(state, &[]).1
}

#[test]
fn changes_save__save_on_a_file_edited_in_the_file_page_writes_it_and_its_row_then_has_a_box_that_can_be_ticked() {
    let repository = committed();
    let (mut state, driver) = edited(&repository, &[("a.txt", "a, edited\n")]);
    assert!(
        find(&drawn(&state), Role::CheckBox, "a.txt").is_disabled(),
        "not saved, it cannot be staged"
    );
    let intents = Window::new(ThemeName::Dark, WIDE).click(&state, Role::Button, "Save");
    assert!(
        intents.contains(&Intent::FileView(FileViewIntent::Save(String::from("a.txt")))),
        "{intents:?}"
    );
    for intent in intents {
        driver.intent(&mut state, intent);
    }
    until_ended(&driver, &mut state);
    assert_eq!(on_disk(&repository, "a.txt"), "a, edited\n");
    assert!(!is_unsaved(&state, "a.txt"));
    assert!(!find(&drawn(&state), Role::CheckBox, "a.txt").is_disabled());
}

#[test]
fn changes_save__save_all_with_two_unsaved_files_writes_both_and_logs_two_write_lines() {
    let repository = committed();
    let (mut state, driver) = edited(&repository, &[("a.txt", "a, edited\n"), ("b.txt", "b, edited\n")]);
    let intents = Window::new(ThemeName::Dark, WIDE).click(&state, Role::Button, "Save all");
    assert!(
        intents.contains(&Intent::FileView(FileViewIntent::SaveAll)),
        "{intents:?}"
    );
    for intent in intents {
        driver.intent(&mut state, intent);
    }
    until_ended(&driver, &mut state);
    let writes: Vec<String> = logged(&state)
        .into_iter()
        .filter(|line| line.starts_with("write "))
        .collect();
    let ends: Vec<bool> = writes
        .iter()
        .map(|line| line.ends_with("a.txt") || line.ends_with("b.txt"))
        .collect();
    assert_eq!(ends, [true, true], "{writes:?}");
    assert_eq!(on_disk(&repository, "a.txt"), "a, edited\n");
    assert_eq!(on_disk(&repository, "b.txt"), "b, edited\n");
    assert!(
        !is_unsaved(&state, "a.txt") && !is_unsaved(&state, "b.txt"),
        "the dots go"
    );
    assert!(!has(&drawn(&state), Role::Button, "Save all"));
}

#[test]
fn changes_save__a_file_saved_that_ends_up_as_in_head_leaves_the_list() {
    let repository = committed();
    std::fs::write(repository.path().join("a.txt"), "a, changed on disk\n").unwrap();
    let (mut state, driver) = edited(&repository, &[("a.txt", "a\n")]);
    assert!(has(&drawn(&state), Role::CheckBox, "a.txt"));
    driver.intent(&mut state, Intent::FileView(FileViewIntent::SaveAll));
    until_ended(&driver, &mut state);
    assert_eq!(on_disk(&repository, "a.txt"), "a\n");
    assert!(!has(&drawn(&state), Role::CheckBox, "a.txt"));
}

#[test]
fn changes_discard_all__the_files_only_an_editor_has_changed_keep_their_edits_not_saved() {
    let repository = committed();
    std::fs::write(repository.path().join("a.txt"), "a, changed on disk\n").unwrap();
    let (mut state, driver) = edited(&repository, &[("b.txt", "b, edited\n")]);
    let intents = Window::new(ThemeName::Dark, WIDE).click(&state, Role::Button, "Discard all changes");
    for intent in intents {
        driver.intent(&mut state, intent);
    }
    until_ended(&driver, &mut state);
    let lines = logged(&state);
    assert!(
        !lines.is_empty() && lines.iter().all(|line| !line.contains("b.txt")),
        "{lines:?}"
    );
    assert_eq!(on_disk(&repository, "a.txt"), "a\n");
    assert!(is_unsaved(&state, "b.txt"), "the edits of b.txt are kept");
    assert_eq!(on_disk(&repository, "b.txt"), "b\n");
    let text = state
        .file_pages
        .pages
        .iter()
        .find(|page| page.path == "b.txt")
        .and_then(|page| page.document())
        .map(|document| document.buffer().text().to_owned());
    assert_eq!(text.as_deref(), Some("b, edited\n"));
}

#[test]
fn changes_pane__save_all_is_on_the_header_only_while_a_file_has_edits_not_saved() {
    let repository = committed();
    std::fs::write(repository.path().join("a.txt"), "a, changed on disk\n").unwrap();
    let (state, _driver) = edited(&repository, &[]);
    assert!(has(&drawn(&state), Role::CheckBox, "a.txt"));
    assert!(!has(&drawn(&state), Role::Button, "Save all"));
    let (state, _driver) = edited(&repository, &[("b.txt", "b, edited\n")]);
    assert!(has(&drawn(&state), Role::Button, "Save all"));
}

#[test]
fn changes_unsaved__a_file_changed_for_git_with_edits_not_saved_has_a_dot_after_its_name_and_save() {
    let repository = committed();
    std::fs::write(repository.path().join("a.txt"), "a, changed on disk
").unwrap();
    let (state, _driver) = edited(&repository, &[("a.txt", "a, typed
")]);
    let tree = drawn(&state);
    assert!(says(&tree, views::text::changes::UNSAVED), "the dot");
    assert!(has(&tree, Role::Button, "Save"));
    assert!(!find(&tree, Role::CheckBox, "a.txt").is_disabled(), "changed for git, it can be staged");
}

#[test]
fn changes_unsaved__a_file_git_ignores_is_left_out_and_one_it_does_not_is_listed() {
    let mut repository = committed();
    let _ = repository.commit("Ignore the build", &[(".gitignore", "build/
")]);
    std::fs::create_dir_all(repository.path().join("build")).unwrap();
    std::fs::write(repository.path().join("build/out.txt"), "out
").unwrap();
    let (state, _driver) = edited(&repository, &[("build/out.txt", "typed
"), ("b.txt", "b, typed
")]);
    let tree = drawn(&state);
    assert!(has(&tree, Role::CheckBox, "b.txt"));
    assert!(!has(&tree, Role::CheckBox, "build/out.txt"));
}
