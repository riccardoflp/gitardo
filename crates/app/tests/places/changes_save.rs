//! **Save** and **Save all** on the Changes page (`changes.save`): the edits typed in File pages and not saved yet
//! written to disk, so that git sees them, and the list then showing the files as git sees them.

use app::store::State;
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use ui::editor::Command;
use views::Intent;
use views::changes::ChangesIntent;
use views::file_view::{FilePageIntent, FileTarget, FileViewIntent};

use super::support::{OperationsDriver, Window, find, has};

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
