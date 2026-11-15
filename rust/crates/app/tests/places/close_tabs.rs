//! Closing a tab with a File page whose edits are not saved (`repo.close-tab`): it asks what closing that page asks,
//! and keeping such a tab when `repositories.keepOpen` is off (`repo.keep-open`), over three clones made for each test.

use std::path::PathBuf;

use app::persisted::same_folder;
use app::store::State;
use ui::editor::Command;
use views::Intent;
use views::file_view::{CloseAnswer, FilePageId, FilePageIntent, FileTarget, FileViewIntent};
use views::page::Page;

use super::compare::until_ended;
use super::support::OperationsDriver;
use super::tabs::three_clones;

/// The text typed in `README.md`.
const TYPED: &str = "Gitardo, edited\n";

/// Opens `README.md` of the repository shown in a File page and replaces its text, without saving it.
fn edit_readme(driver: &OperationsDriver, state: &mut State) -> FilePageId {
    driver.intent(
        state,
        Intent::FileView(FileViewIntent::Open(FileTarget::working_tree("README.md"))),
    );
    let id = state
        .file_pages
        .pages
        .iter()
        .find(|page| page.path == "README.md")
        .unwrap()
        .id;
    let commands = vec![Command::SelectAll, Command::Paste(TYPED.to_owned())];
    driver.intent(
        state,
        Intent::FileView(FileViewIntent::Page(id, FilePageIntent::Edit(commands))),
    );
    id
}

/// Answers **Save changes to README.md?**
fn answer(driver: &OperationsDriver, state: &mut State, answer: CloseAnswer) {
    driver.intent(state, Intent::FileView(FileViewIntent::AnswerClose(answer)));
}

/// The folders of the tabs open, in their order.
fn tab_folders(state: &State) -> Vec<PathBuf> {
    state.tabs.open.iter().map(|tab| tab.folder.clone()).collect()
}

/// Whether the repository shown is the one of a folder.
fn shows(state: &State, folder: &std::path::Path) -> bool {
    state.shown_folder().is_some_and(|shown| same_folder(shown, folder))
}

/// The text of `README.md` on disk in a clone.
fn readme(clone: &std::path::Path) -> String {
    std::fs::read_to_string(clone.join("README.md")).unwrap()
}

#[test]
fn repo_close_tab__a_page_whose_edits_are_not_saved_asks_and_cancel_keeps_the_tab_and_the_edits() {
    let (_kept, clones, mut state) = three_clones();
    let driver = OperationsDriver::new();
    let id = edit_readme(&driver, &mut state);
    driver.intent(&mut state, Intent::CloseTab(0));
    assert_eq!(
        state.file_pages.closing,
        Some(id),
        "Save changes to README.md? is asked"
    );
    assert_eq!(state.page_shown(), Some(Page::File(id)), "over the page with the edits");
    assert_eq!(tab_folders(&state).len(), 3);
    answer(&driver, &mut state, CloseAnswer::Cancel);
    assert_eq!(tab_folders(&state).len(), 3);
    assert!(shows(&state, &clones[0]));
    assert!(
        state
            .file_pages
            .pages
            .iter()
            .any(|page| page.id == id && page.is_unsaved())
    );
    assert_eq!(state.tabs.closing, None);
}

#[test]
fn repo_close_tab__a_tab_not_shown_is_shown_to_ask_and_dont_save_closes_it_with_the_edits_lost() {
    let (_kept, clones, mut state) = three_clones();
    let driver = OperationsDriver::new();
    let id = edit_readme(&driver, &mut state);
    driver.intent(&mut state, Intent::SwitchTab(1));
    driver.intent(&mut state, Intent::CloseTab(0));
    assert!(shows(&state, &clones[0]), "api is shown while it asks");
    assert_eq!(state.file_pages.closing, Some(id));
    answer(&driver, &mut state, CloseAnswer::DontSave);
    assert_eq!(tab_folders(&state).len(), 2);
    assert!(same_folder(&state.tabs.open[0].folder, &clones[1]));
    assert!(shows(&state, &clones[1]), "the tab shown before is shown again");
    assert_eq!(readme(&clones[0]), "Gitardo\n");
}

#[test]
fn repo_close_tab__save_writes_the_file_then_closes_the_tab() {
    let (_kept, clones, mut state) = three_clones();
    let driver = OperationsDriver::new();
    let _id = edit_readme(&driver, &mut state);
    driver.intent(&mut state, Intent::CloseTab(0));
    answer(&driver, &mut state, CloseAnswer::Save);
    until_ended(&driver, &mut state);
    assert_eq!(readme(&clones[0]), TYPED);
    assert_eq!(tab_folders(&state).len(), 2);
    assert!(shows(&state, &clones[1]), "the tab on the right of api is shown");
}

#[test]
fn repo_close_tab__close_other_tabs_asks_for_the_page_of_api_and_leaves_web_alone() {
    let (_kept, clones, mut state) = three_clones();
    let driver = OperationsDriver::new();
    let id = edit_readme(&driver, &mut state);
    driver.intent(&mut state, Intent::SwitchTab(1));
    driver.intent(&mut state, Intent::CloseOtherTabs(1));
    assert_eq!(state.file_pages.closing, Some(id));
    answer(&driver, &mut state, CloseAnswer::DontSave);
    assert_eq!(tab_folders(&state).len(), 1);
    assert!(same_folder(&state.tabs.open[0].folder, &clones[1]));
    assert!(shows(&state, &clones[1]));
}

#[test]
fn repo_keep_open__with_the_setting_off_a_tab_with_a_page_not_saved_keeps_it() {
    let (_kept, _clones, mut state) = three_clones();
    state.settings.keep_open = false;
    let driver = OperationsDriver::new();
    let id = edit_readme(&driver, &mut state);
    driver.intent(&mut state, Intent::SwitchTab(1));
    driver.intent(&mut state, Intent::SwitchTab(0));
    assert!(
        state
            .file_pages
            .pages
            .iter()
            .any(|page| page.id == id && page.is_unsaved()),
        "the edits are still there"
    );
    assert!(state.pages.open.contains(&Page::File(id)));
}
