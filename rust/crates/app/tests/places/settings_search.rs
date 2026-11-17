//! The search of the settings page and its lists of the settings changed and experimental (`settings.search`,
//! `settings.sections`), with settings files written for each test.

use app::store::State;
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::Intent;
use views::page::Page;
use views::settings::{SettingsAnchor, SettingsIntent, SettingsList};
use views::text::settings::{SettingsSection, settings};

use super::pointer::{apply, bounds};
use super::support::{Window, typed};
use super::{repository_with_history, showing};

/// A window tall enough to draw every setting.
const TALL: (u32, u32) = (1280, 12_000);

/// A window showing a repository, with the user's settings file written and read, and the settings page open.
fn settings_page(user: &str) -> (State, testkit::Repository) {
    let (repository, _remote) = repository_with_history();
    let file = repository.path().parent().unwrap().join("user-settings.json");
    std::fs::write(&file, user).unwrap();
    let mut state = showing(repository.path().to_path_buf());
    state.settings = app::settings::read(&file, None);
    state.user_settings_file = Some(file);
    apply(&mut state, vec![Intent::ShowPage(Page::Settings)]);
    (state, repository)
}

/// The button of a section, or of **Changed** or **Experimental**: the buttons of the column say whether they are
/// marked.
fn column_button<'tree>(tree: &'tree TreeUpdate, name: &str) -> &'tree platform::accesskit::Node {
    let found = tree
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some(name) && node.is_selected().is_some());
    &found.unwrap().1
}

/// The keys of the settings the list shows.
fn keys_listed(tree: &TreeUpdate) -> Vec<&'static str> {
    settings()
        .map(|info| info.key)
        .filter(|key| super::support::says(tree, key))
        .collect()
}

/// The names of the sections over the list.
fn headings(tree: &TreeUpdate) -> Vec<&'static str> {
    SettingsSection::ALL
        .iter()
        .map(|section| section.name())
        .filter(|name| super::support::has(tree, Role::Heading, name))
        .collect()
}

/// Types in the search of the page, which has the keyboard, and hands what it asked to the store.
fn search(window: &mut Window, state: &mut State, text: &str) {
    let _ = window.draw(state, &[]);
    let _ = window.draw(state, &[]);
    let (intents, _) = window.draw(state, &[typed(text)]);
    assert_eq!(intents, vec![Intent::Settings(SettingsIntent::Search(text.to_owned()))]);
    apply(state, intents);
}

#[test]
fn settings_sections__with_two_settings_set_their_sections_each_say_1_and_changed_lists_them_alone() {
    let user = "{\n  \"graph.firstParent\": true,\n  \"pull.rebase\": false\n}\n";
    let (mut state, _repository) = settings_page(user);
    let mut window = Window::new(ThemeName::Dark, TALL);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(column_button(&tree, "Timeline").value(), Some("1"));
    assert_eq!(column_button(&tree, "Pull and commits").value(), Some("1"));
    assert_eq!(column_button(&tree, "General").value(), Some(""));
    let intents = window.click(&state, Role::Button, "Changed");
    assert_eq!(
        intents,
        vec![Intent::Settings(SettingsIntent::Show(SettingsList::Changed))]
    );
    apply(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(keys_listed(&tree), vec!["graph.firstParent", "pull.rebase"]);
    assert_eq!(column_button(&tree, "Changed").is_selected(), Some(true));
    let again = window.click(&state, Role::Button, "Changed");
    apply(&mut state, again);
    let (_, tree) = window.draw(&state, &[]);
    assert!(keys_listed(&tree).len() > 40, "clicked again, the sections are back");
}

#[test]
fn settings_sections__experimental_lists_conflicts_auto_resolve_and_conflicts_ai_alone() {
    let (mut state, _repository) = settings_page("{}\n");
    let mut window = Window::new(ThemeName::Dark, TALL);
    let intents = window.click(&state, Role::Button, "Experimental");
    apply(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(keys_listed(&tree), vec!["conflicts.autoResolve", "conflicts.ai"]);
    assert_eq!(
        headings(&tree),
        Vec::<&str>::new(),
        "the settings are listed without their sections"
    );
}

#[test]
fn settings_sections__a_value_of_the_wrong_type_in_a_file_marks_its_section_in_the_color_of_errors() {
    let (state, _repository) = settings_page("{\n  \"graph.pageSize\": \"many\"\n}\n");
    let mut window = Window::new(ThemeName::Dark, (1280, 720));
    let (_, tree) = window.draw(&state, &[]);
    let timeline = column_button(&tree, "Timeline");
    assert!(
        timeline.value().is_some_and(|said| said.contains("not taken")),
        "{:?}",
        timeline.value()
    );
    let general = column_button(&tree, "General");
    assert!(!general.value().is_some_and(|said| said.contains("not taken")));
}

#[test]
fn settings_search__blame_leaves_the_settings_of_files_and_blame_alone_and_dims_every_other_section() {
    let (mut state, _repository) = settings_page("{}\n");
    let mut window = Window::new(ThemeName::Dark, TALL);
    search(&mut window, &mut state, "blame");
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(headings(&tree), vec!["Files and blame"]);
    for section in SettingsSection::ALL {
        let button = column_button(&tree, section.name());
        let is_files = section == SettingsSection::FilesAndBlame;
        assert_eq!(button.is_disabled(), !is_files, "{}", section.name());
    }
    let listed = keys_listed(&tree);
    let infos = settings().filter(|info| listed.contains(&info.key));
    assert!(
        infos
            .into_iter()
            .all(|info| info.section == Some(SettingsSection::FilesAndBlame))
    );
}

#[test]
fn settings_search__escape_brings_back_every_setting_and_the_section_marked_before() {
    let (mut state, _repository) = settings_page("{}\n");
    let mut window = Window::new(ThemeName::Dark, (1280, 720));
    let worktrees = Intent::Settings(SettingsIntent::Reveal(SettingsAnchor::Section(
        SettingsSection::Worktrees,
    )));
    apply(&mut state, vec![worktrees]);
    let _ = window.draw(&state, &[]);
    let (_, tree) = window.draw(&state, &[]);
    let top = bounds(&tree, Role::Heading, "Worktrees").y;
    search(&mut window, &mut state, "blame");
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(headings(&tree), vec!["Files and blame"]);
    let escape = InputEvent::KeyPressed(KeyEvent::press(Key::Named(NamedKey::Escape), Modifiers::NONE));
    let (intents, _) = window.draw(&state, &[escape]);
    assert_eq!(intents, vec![Intent::Settings(SettingsIntent::Search(String::new()))]);
    apply(&mut state, intents);
    let _ = window.draw(&state, &[]);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(column_button(&tree, "Worktrees").is_selected(), Some(true));
    assert!((bounds(&tree, Role::Heading, "Worktrees").y - top).abs() < 1.0);
}
