//! Every value changed on the settings page (`settings.edit`) and taken back to its default (`settings.reset`), with
//! settings files written for each test.

use std::path::{Path, PathBuf};

use app::store::State;
use platform::accesskit::{Role, TreeUpdate};
use render::{Rect, ThemeName};
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey, Point};
use views::Intent;
use views::page::Page;
use views::settings::{Identity, SettingChange, SettingValue, SettingsAnchor, SettingsIntent, SettingsTarget};

use super::compare::until_ended;
use super::pointer::{apply, bounds, click_at, middle, rect};
use super::support::{OperationsDriver, Window, has, says_in, shortcut, typed};
use super::{repository_with_history, showing};

/// The size of the window.
const SIZE: (u32, u32) = (1280, 720);

/// A window showing a repository, with the user's settings file written and read, and the settings page open at a
/// setting.
fn settings_page(user: &str, at: &'static str) -> (State, testkit::Repository, PathBuf) {
    let (repository, _remote) = repository_with_history();
    let file = repository.path().parent().unwrap().join("user-settings.json");
    std::fs::write(&file, user).unwrap();
    let mut state = showing(repository.path().to_path_buf());
    state.user_settings_file = Some(file.clone());
    read_again(&mut state);
    let reveal = Intent::Settings(SettingsIntent::Reveal(SettingsAnchor::Setting(at)));
    apply(&mut state, vec![Intent::ShowPage(Page::Settings), reveal]);
    (state, repository, file)
}

/// Reads the settings files again, as the watcher does when one changes.
fn read_again(state: &mut State) {
    let user = state.user_settings_file.clone().unwrap();
    let repository = state
        .shown_folder()
        .map(|folder| app::settings::repository_settings_file(folder));
    state.settings = app::settings::read(&user, repository.as_deref());
}

/// A key pressed alone.
fn press(key: NamedKey) -> InputEvent {
    InputEvent::KeyPressed(KeyEvent::press(Key::Named(key), Modifiers::NONE))
}

/// Draws the window twice, so that the page is laid out and scrolled where it was asked, and gives its tree.
fn settled(window: &mut Window, state: &State) -> TreeUpdate {
    let _ = window.draw(state, &[]);
    window.draw(state, &[]).1
}

/// Clicks in the field of a role and a name, selects its text, types over it and presses keys; gives the intents.
fn type_over(
    window: &mut Window,
    state: &State,
    (role, name): (Role, &str),
    (text, then): (&str, &[InputEvent]),
) -> Vec<Intent> {
    let mut intents = window.click(state, role, name);
    intents.extend(window.draw(state, &[shortcut('a', Modifiers::CONTROL)]).0);
    intents.extend(window.draw(state, &[typed(text)]).0);
    for event in then {
        intents.extend(window.draw(state, std::slice::from_ref(event)).0);
    }
    intents
}

/// The changes the intents ask to write.
fn written(intents: &[Intent]) -> Vec<SettingChange> {
    intents
        .iter()
        .filter_map(|intent| match intent {
            Intent::SetSetting(change) => Some(change.clone()),
            _ => None,
        })
        .collect()
}

/// A change of the user's file.
fn user_change(key: &'static str, value: SettingValue) -> SettingChange {
    SettingChange {
        key,
        value,
        target: SettingsTarget::User,
    }
}

/// The middle of the field whose value is a text, which the test fails without.
fn field_with(tree: &TreeUpdate, value: &str) -> Point {
    let found = tree
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::TextInput && node.value() == Some(value));
    middle(rect(found.unwrap().1.bounds().unwrap()))
}

/// The middle of **Add** under a field.
fn add_under(tree: &TreeUpdate, field: Point) -> Point {
    let add = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some("Add"))
        .filter_map(|(_, node)| node.bounds().map(rect))
        .filter(|found| found.y > field.y)
        .min_by(|one, other| one.y.total_cmp(&other.y));
    middle(add.unwrap())
}

#[test]
fn settings_edit__500_typed_in_commits_read_at_a_time_and_enter_write_it_and_5_is_refused_with_the_range() {
    let (state, _repository, _file) = settings_page("{}\n", "graph.pageSize");
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let _ = settled(&mut window, &state);
    let field = (Role::TextInput, "100 to 10000");
    let refused = type_over(&mut window, &state, field, ("5", &[press(NamedKey::Enter)]));
    assert_eq!(written(&refused), Vec::new());
    let (_, tree) = window.draw(&state, &[]);
    assert!(says_in(&tree, "A whole number from 100 to 10000"));
    let taken = type_over(&mut window, &state, field, ("500", &[press(NamedKey::Enter)]));
    assert_eq!(
        written(&taken),
        vec![user_change("graph.pageSize", SettingValue::Number(500))]
    );
}

#[test]
fn settings_edit__a_typed_value_is_written_once_when_the_field_loses_the_focus_not_at_each_key() {
    let (state, _repository, _file) = settings_page("{}\n", "worktrees.folder");
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let _ = settled(&mut window, &state);
    let mut intents = type_over(&mut window, &state, (Role::TextInput, "${repo}"), ("../t", &[]));
    intents.extend(window.draw(&state, &[typed("r")]).0);
    intents.extend(window.draw(&state, &[typed("ees")]).0);
    assert_eq!(written(&intents), Vec::new(), "nothing is written while typing");
    let elsewhere = window.click(&state, Role::Button, "General");
    let folder = SettingValue::Text(String::from("../trees"));
    assert_eq!(written(&elsewhere), vec![user_change("worktrees.folder", folder)]);
    let (again, _) = window.draw(&state, &[]);
    assert_eq!(written(&again), Vec::new(), "once");
}

#[test]
fn settings_edit__a_folder_added_to_folders_to_link_writes_the_list_with_it_in_the_order_shown() {
    let (mut state, _repository, file) = settings_page(
        "{\n  \"worktrees.linkFolders\": [\"node_modules\"]\n}\n",
        "worktrees.linkFolders",
    );
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = settled(&mut window, &state);
    let entry = field_with(&tree, "node_modules");
    let added = click_at(&mut window, &state, add_under(&tree, entry), 1);
    let with_empty = SettingValue::Strings(vec![String::from("node_modules"), String::new()]);
    assert_eq!(written(&added), vec![user_change("worktrees.linkFolders", with_empty)]);
    std::fs::write(&file, "{\n  \"worktrees.linkFolders\": [\"node_modules\", \"\"]\n}\n").unwrap();
    read_again(&mut state);
    let tree = settled(&mut window, &state);
    window.assert_snapshot("settings-lists-dark");
    let empty = Point::new(entry.x, entry.y + 36.0);
    let _ = field_with(&tree, "");
    let _ = click_at(&mut window, &state, empty, 1);
    let mut intents = window.draw(&state, &[typed("target")]).0;
    intents.extend(window.draw(&state, &[press(NamedKey::Enter)]).0);
    let list = SettingValue::Strings(vec![String::from("node_modules"), String::from("target")]);
    assert_eq!(written(&intents), vec![user_change("worktrees.linkFolders", list)]);
}

#[test]
fn settings_edit__an_identity_added_to_the_table_writes_identities_with_its_name_its_email_and_its_signing_key() {
    let (mut state, _repository, file) = settings_page("{}\n", "identities");
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = settled(&mut window, &state);
    let title = super::pointer::said_at(&tree, "Identities", Rect::new(0.0, 0.0, 4_000.0, 4_000.0));
    let added = click_at(&mut window, &state, add_under(&tree, middle(title)), 1);
    let one = SettingValue::Identities(vec![Identity::default()]);
    assert_eq!(written(&added), vec![user_change("identities", one)]);
    std::fs::write(&file, "{\n  \"identities\": [{ \"name\": \"\", \"email\": \"\" }]\n}\n").unwrap();
    read_again(&mut state);
    let _ = settled(&mut window, &state);
    let mut intents = type_over(&mut window, &state, (Role::TextInput, "Name"), ("Ada Lovelace", &[]));
    intents.extend(type_over(
        &mut window,
        &state,
        (Role::TextInput, "Email"),
        ("ada@example.com", &[]),
    ));
    let key = [press(NamedKey::Enter)];
    intents.extend(type_over(
        &mut window,
        &state,
        (Role::TextInput, "Signing key"),
        ("ABCD1234", &key),
    ));
    let ada = Identity {
        name: String::from("Ada Lovelace"),
        email: String::from("ada@example.com"),
        signing_key: String::from("ABCD1234"),
    };
    let last = written(&intents).pop();
    assert_eq!(
        last,
        Some(user_change("identities", SettingValue::Identities(vec![ada])))
    );
}

#[test]
fn settings_reset__a_setting_no_file_sets_has_no_reset_button() {
    let (state, _repository, _file) = settings_page("{\n  \"graph.showStashes\": false\n}\n", "graph.pageSize");
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = settled(&mut window, &state);
    assert!(has(&tree, Role::Button, "Back to the default: Show the stashes"));
    assert!(!has(&tree, Role::Button, "Back to the default: Commits read at a time"));
    let stashes = bounds(&tree, Role::Button, "Back to the default: Show the stashes");
    assert!(stashes.width > 0.0);
}

#[test]
fn settings_reset__with_pull_rebase_in_both_files_reset_in_repository_takes_it_out_of_the_repositorys_file_alone() {
    let user = "{\n  // Mine.\n  \"pull.rebase\": false\n}\n";
    let (mut state, repository, file) = settings_page(user, "pull.rebase");
    let repository_file = repository_file(repository.path());
    std::fs::create_dir_all(repository_file.parent().unwrap()).unwrap();
    std::fs::write(
        &repository_file,
        "{\n  \"pull.rebase\": true,\n  \"pull.autostash\": false\n}\n",
    )
    .unwrap();
    read_again(&mut state);
    let driver = OperationsDriver::new();
    driver.intent(&mut state, Intent::ChooseSettingsFile(SettingsTarget::Repository));
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let _ = settled(&mut window, &state);
    for intent in window.click(&state, Role::Button, "Back to the default: Pull with rebase") {
        driver.intent(&mut state, intent);
    }
    until_ended(&driver, &mut state);
    assert_eq!(
        std::fs::read_to_string(&repository_file).unwrap(),
        "{\n  \"pull.autostash\": false\n}\n"
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap(), user);
    read_again(&mut state);
    let tree = settled(&mut window, &state);
    assert!(!has(&tree, Role::Button, "Back to the default: Pull with rebase"));
    assert!(says_in(&tree, "From user settings"));
}

/// The repository's settings file.
fn repository_file(folder: &Path) -> PathBuf {
    folder.join(".gitardo").join("settings.json")
}
