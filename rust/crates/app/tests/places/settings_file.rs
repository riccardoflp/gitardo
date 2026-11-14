//! The settings files opened in a File page from **Edit in settings.json** of the settings page (`settings.edit`,
//! `worktree-setup.edit-settings-file`), on repositories made for each test.

use app::store::State;
use platform::accesskit::Role;
use render::ThemeName;
use views::Intent;
use views::page::Page;
use views::settings::SettingsTarget;

use super::compare::until_ended;
use super::file_view::shown;
use super::showing;
use super::support::{OperationsDriver, Window};

/// The size of the window of the settings page.
const WIDE: (u32, u32) = (1000, 520);

/// The settings page of a window showing a repository, with the user's settings file beside the repository.
fn settings_page(repository: &testkit::Repository, user: &str) -> (State, OperationsDriver, std::path::PathBuf) {
    let file = repository.path().parent().unwrap().join("user-settings.json");
    std::fs::write(&file, user).unwrap();
    let mut state = showing(repository.path().to_path_buf());
    state.user_settings_file = Some(file.clone());
    let driver = OperationsDriver::new();
    driver.intent(&mut state, Intent::ShowPage(Page::Settings));
    (state, driver, file)
}

/// Clicks **Edit in settings.json** and waits for the operation to end.
fn edit_in_settings_file(state: &mut State, driver: &OperationsDriver) {
    let mut window = Window::new(ThemeName::Dark, WIDE);
    for intent in window.click(state, Role::Button, "Edit in settings.json") {
        driver.intent(state, intent);
    }
    until_ended(driver, state);
}

#[test]
fn settings_edit__edit_in_settings_json_opens_the_users_file_in_a_file_page() {
    let repository = testkit::Repository::new();
    let (mut state, driver, file) = settings_page(&repository, "{\n  \"theme\": \"dark\"\n}\n");
    edit_in_settings_file(&mut state, &driver);
    let page = shown(&state);
    assert_eq!(page.path, file.display().to_string());
    assert_eq!(page.title, "user-settings.json");
}

#[test]
fn worktree_setup_edit_settings_file__with_repository_settings_chosen_the_file_is_made_with_an_empty_object() {
    let repository = testkit::Repository::new();
    let (mut state, driver, _file) = settings_page(&repository, "{}\n");
    driver.intent(&mut state, Intent::ChooseSettingsFile(SettingsTarget::Repository));
    edit_in_settings_file(&mut state, &driver);
    let made = repository.path().join(".gitardo").join("settings.json");
    assert_eq!(std::fs::read_to_string(made).unwrap(), "{}\n");
    assert_eq!(shown(&state).path, ".gitardo/settings.json");
}
