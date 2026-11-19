//! The options of the next commit (`commit.options`): the button beside **Commit** and its menu, the line under the
//! commit box, and the commits they make, on repositories made for each test. Each repository sets how git signs in its
//! own configuration, since the window reads git with the configuration of whoever runs the tests.

use std::path::{Path, PathBuf};

use app::store::{Message, State};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::changes::ChangesIntent;

use super::support::{self, OperationsDriver, Window, find, has};

/// The size of the window.
const WIDE: (u32, u32) = (1000, 520);

/// A repository with one commit and `a.txt` staged, the identity `Ada <ada@example.com>` and a signing key of its own,
/// which does not sign every commit.
pub(super) fn staged() -> Repository {
    let mut repository = Repository::new();
    for (key, value) in [
        ("user.name", "Ada"),
        ("user.email", "ada@example.com"),
        ("commit.gpgSign", "false"),
        ("gpg.format", "openpgp"),
        ("user.signingKey", "3AA5C34371567BD2"),
    ] {
        let _ = repository.git(&["config", key, value]);
    }
    let _ = repository.commit("Add the files", &[("a.txt", "a\n")]);
    stage(&repository, "a, changed\n");
    repository
}

/// Changes `a.txt` and stages it.
pub(super) fn stage(repository: &Repository, text: &str) {
    std::fs::write(repository.path().join("a.txt"), text).unwrap();
    let _ = repository.git(&["add", "a.txt"]);
}

/// A window showing the folders, the first of them shown, with its Changes page open.
pub(super) fn changes_page(folders: Vec<PathBuf>) -> (State, OperationsDriver) {
    let mut state = super::window();
    let _ = support::run(&mut state, Message::Started { folders });
    let driver = OperationsDriver::new();
    driver.intent(&mut state, Intent::Changes(ChangesIntent::Open));
    (state, driver)
}

/// Reduces the intents of a frame.
fn apply(driver: &OperationsDriver, state: &mut State, intents: Vec<Intent>) {
    for intent in intents {
        driver.intent(state, intent);
    }
}

/// Opens the menu of the options from the button of the Changes page, drawn after the one of the side column, and
/// draws it.
pub(super) fn open_menu(driver: &OperationsDriver, state: &mut State, window: &mut Window) {
    let (_, tree) = window.draw(state, &[]);
    let buttons = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some("Commit options"));
    let mut rects = buttons.filter_map(|(_, node)| node.bounds().map(super::pointer::rect));
    let center = super::pointer::middle(rects.next_back().unwrap());
    let intents = window.click_at(state, center, 1);
    apply(driver, state, intents);
    let _ = window.draw(state, &[]);
}

/// Opens the menu of the options and chooses one of its entries.
fn choose(driver: &OperationsDriver, state: &mut State, entry: &str) {
    let mut window = Window::new(ThemeName::Dark, WIDE);
    open_menu(driver, state, &mut window);
    let intents = window.click(state, Role::MenuItem, entry);
    apply(driver, state, intents);
}

/// Types a message and commits, then waits for the commit to end; gives the command of git it ran last.
pub(super) fn commit(driver: &OperationsDriver, state: &mut State, message: &str) -> String {
    driver.intent(state, Intent::Changes(ChangesIntent::Type(String::from(message))));
    driver.intent(state, Intent::Changes(ChangesIntent::Commit));
    for _ in 0..8 {
        if matches!(driver.next(state), Message::FeatureEnded { .. }) {
            let lines: Vec<String> = state.log.entries().map(|entry| entry.line.clone()).collect();
            let mut commits = lines.iter().filter(|line| line.starts_with("git commit"));
            return commits.next_back().cloned().unwrap_or_default();
        }
    }
    panic!("the commit does not end");
}

/// The line under the commit box, while it says something.
pub(super) fn line_under_the_box(state: &State) -> Option<String> {
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(state, &[]);
    let words = [
        "Signed",
        "Not signed",
        "Signed-off-by",
        "No Signed-off-by",
        "Hooks skipped",
        "With ",
    ];
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button)
        .filter_map(|(_, node)| node.label())
        .find(|label| words.iter().any(|word| label.starts_with(word)))
        .map(str::to_owned)
}

/// Shows another repository of the workspace, and reads it.
fn show(driver: &OperationsDriver, state: &mut State, folder: &Path) {
    driver.intent(state, Intent::SelectRepository(folder.to_path_buf()));
}

#[test]
fn commit_options__with_gpgsign_true_sign_is_ticked_and_unticked_says_not_signed_and_the_commit_is_not_signed() {
    let repository = staged();
    let _ = repository.git(&["config", "commit.gpgSign", "true"]);
    let (mut state, driver) = changes_page(vec![repository.path().to_path_buf()]);
    assert_eq!(line_under_the_box(&state), None, "nothing differs from where it starts");
    choose(&driver, &mut state, "Sign this commit");
    assert_eq!(line_under_the_box(&state).as_deref(), Some("Not signed"));
    let ran = commit(&driver, &mut state, "Fix the parser");
    assert_eq!(ran, "git commit --no-gpg-sign -m \"Fix the parser\"");
    let object = repository.git(&["cat-file", "commit", "HEAD"]);
    assert!(!object.contains("gpgsig"), "{object}");
}

#[test]
fn commit_options__with_ssh_and_no_signing_key_sign_cannot_be_ticked_and_switch_identity_is_offered() {
    let repository = staged();
    let _ = repository.git(&["config", "gpg.format", "ssh"]);
    let _ = repository.git(&["config", "user.signingKey", ""]);
    let (mut state, driver) = changes_page(vec![repository.path().to_path_buf()]);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    open_menu(&driver, &mut state, &mut window);
    let (_, tree) = window.draw(&state, &[]);
    assert!(find(&tree, Role::MenuItem, "Sign this commit").is_disabled());
    assert!(has(&tree, Role::MenuItem, "Switch identity\u{2026}"));
    let intents = window.click(&state, Role::MenuItem, "Sign this commit");
    assert!(intents.is_empty(), "{intents:?}");
}

#[test]
fn commit_options__commit_sign_off_in_the_settings_of_a_repository_starts_ticked_in_that_repository_only() {
    let first = staged();
    let second = staged();
    let settings = app::settings::repository_settings_file(first.path());
    std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
    std::fs::write(&settings, "{ \"commit.signOff\": true }\n").unwrap();
    let user = first.path().parent().unwrap().join("user-settings.json");
    std::fs::write(&user, "{}\n").unwrap();
    let folders = vec![first.path().to_path_buf(), second.path().to_path_buf()];
    let (mut state, driver) = changes_page(folders);
    for (repository, is_signed_off) in [(&first, true), (&second, false)] {
        show(&driver, &mut state, repository.path());
        let file = app::settings::repository_settings_file(repository.path());
        let read = app::settings::read(&user, Some(&file));
        let _ = support::run(&mut state, Message::SettingsRead(Box::new(read)));
        let ran = commit(&driver, &mut state, "Fix the parser");
        assert_eq!(ran.contains("--signoff"), is_signed_off, "{ran}");
    }
    let message = first.git(&["log", "-1", "--format=%B"]);
    assert!(message.contains("Signed-off-by: Ada <ada@example.com>"), "{message}");
}

#[test]
fn commit_options__after_a_commit_with_skip_the_hooks_the_next_commit_runs_the_hooks_again() {
    let repository = staged();
    let hooks = repository.path().parent().unwrap().join("hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    std::fs::write(hooks.join("pre-commit"), "#!/bin/sh\necho ran >> hook-ran.txt\n").unwrap();
    let hooks_path = hooks.to_string_lossy().replace('\\', "/");
    let _ = repository.git(&["config", "core.hooksPath", &hooks_path]);
    let ran_file = repository.path().join("hook-ran.txt");
    let _ = repository.git(&["config", "core.excludesFile", ""]);
    std::fs::write(
        repository.path().join(".git").join("info").join("exclude"),
        "hook-ran.txt\n",
    )
    .unwrap();
    let (mut state, driver) = changes_page(vec![repository.path().to_path_buf()]);
    choose(&driver, &mut state, "Skip the hooks");
    assert_eq!(line_under_the_box(&state).as_deref(), Some("Hooks skipped"));
    let ran = commit(&driver, &mut state, "Fix the parser");
    assert_eq!(ran, "git commit --no-verify -m \"Fix the parser\"");
    assert!(!ran_file.exists(), "the hook did not run");
    assert_eq!(line_under_the_box(&state), None, "the box is back where it starts");
    stage(&repository, "a, changed again\n");
    driver.intent(&mut state, Intent::Refresh);
    let ran = commit(&driver, &mut state, "Fix the parser again");
    assert_eq!(ran, "git commit -m \"Fix the parser again\"");
    assert!(ran_file.exists(), "the hook ran");
}

#[test]
fn commit_options__the_repository_shown_changed_and_back_keeps_the_options_ticked() {
    let first = staged();
    let second = staged();
    let folders = vec![first.path().to_path_buf(), second.path().to_path_buf()];
    let (mut state, driver) = changes_page(folders);
    show(&driver, &mut state, first.path());
    choose(&driver, &mut state, "Skip the hooks");
    choose(&driver, &mut state, "Add Signed-off-by");
    let said = Some(String::from("Signed-off-by \u{b7} Hooks skipped"));
    assert_eq!(line_under_the_box(&state), said);
    show(&driver, &mut state, second.path());
    assert_eq!(line_under_the_box(&state), None, "the other repository keeps its own");
    show(&driver, &mut state, first.path());
    assert_eq!(line_under_the_box(&state), said);
}

#[test]
fn commit_options__the_menu_open_matches_its_snapshots_in_the_dark_and_the_light_theme() {
    let repository = staged();
    for (theme, name) in [(ThemeName::Dark, "dark"), (ThemeName::Light, "light")] {
        let (mut state, driver) = changes_page(vec![repository.path().to_path_buf()]);
        let mut window = Window::new(theme, WIDE);
        open_menu(&driver, &mut state, &mut window);
        window.assert_snapshot(&format!("commit-options-menu-{name}"));
    }
}
