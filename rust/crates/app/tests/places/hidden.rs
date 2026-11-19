//! The files hidden from git, under the changes and in the Files page (`changes.skipped-list`, `changes.skip`,
//! `changes.unskip`).

use model::contract::{FeatureId, Target};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::changes::{BesideFile, ChangesIntent};
use views::diff::{DiffFile, DiffTarget, DiffVersion};
use views::feature::FeatureIntent;
use views::text::changes as words;

use super::changes_beside::{apply, changes_page, until_ended};
use super::changes_menu::{choose, menu_of, name_of};
use super::files_page::files_page;
use super::support::{Window, has, says};

/// A window wide enough for the whole Changes page.
const WIDE: (u32, u32) = (1200, 640);

/// A repository with `config/local.json` and `a.txt` committed, the first changed on disk and hidden from git with
/// `--skip-worktree`, the second changed.
fn hidden() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.git(&["config", "user.name", "Ada"]);
    let _ = repository.git(&["config", "user.email", "ada@example.com"]);
    let _ = repository.commit("Add the settings", &[("config/local.json", "{}\n"), ("a.txt", "a\n")]);
    let _ = repository.git(&["update-index", "--skip-worktree", "--", "config/local.json"]);
    std::fs::write(repository.path().join("config/local.json"), "{ \"port\": 1 }\n").unwrap();
    std::fs::write(repository.path().join("a.txt"), "a, changed\n").unwrap();
    repository
}

#[test]
fn changes_skipped_list__a_file_skipped_is_listed_under_hidden_from_git_1_and_not_in_the_changes() {
    let repository = hidden();
    let (state, _driver) = changes_page(&repository);
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    assert!(says(&tree, "Hidden from git 1"));
    assert!(says(&tree, words::SKIP_WORKTREE));
    assert!(has(&tree, Role::Button, "config/local.json"));
    assert!(
        !has(&tree, Role::CheckBox, "config/local.json"),
        "not in the list of the changes"
    );
    assert!(says(&tree, "Changes 1"));
}

#[test]
fn changes_skipped_list__a_click_on_a_file_changed_on_disk_opens_its_diff_between_head_and_the_disk() {
    let repository = hidden();
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let intents = window.click(&state, Role::Button, "config/local.json");
    let _ = tree;
    let diff = DiffTarget::file(
        DiffVersion::Head,
        DiffVersion::WorkingTree,
        DiffFile::same("config/local.json"),
    );
    let opened = Intent::Changes(ChangesIntent::OpenBeside(BesideFile::Diff(Box::new(diff))));
    assert!(intents.contains(&opened), "{intents:?}");
    apply(&driver, &mut state, intents);
}

#[test]
fn changes_skipped_list__with_no_file_hidden_nothing_of_the_group_is_drawn() {
    let repository = super::changes_beside::three_changed();
    let (state, _driver) = changes_page(&repository);
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    assert!(!says(&tree, words::HIDDEN_FROM_GIT));
    assert!(
        !tree
            .nodes
            .iter()
            .any(|(_, node)| node.label().is_some_and(|label| label.starts_with("Hidden from git")))
    );
}

#[test]
fn changes_skip__hide_its_changes_from_git_takes_a_file_out_of_the_changes_and_lists_it_under_hidden_from_git() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the settings", &[("config/local.json", "{}\n")]);
    std::fs::write(repository.path().join("config/local.json"), "{ \"port\": 1 }\n").unwrap();
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let items = menu_of(&mut window, &state, "config/local.json");
    assert!(items.iter().any(|item| item == words::HIDE_FROM_GIT), "{items:?}");
    let intents = window.click(&state, Role::MenuItem, words::HIDE_FROM_GIT);
    let asked = Intent::Feature(FeatureIntent::Run(
        FeatureId("changes.skip"),
        Target::File(String::from("config/local.json")),
    ));
    assert!(intents.contains(&asked), "{intents:?}");
    super::ignore::answer_until_ended(&driver, &mut state, intents, &["skip-worktree"]);
    let status = repository.git(&["status", "--porcelain=v1"]);
    assert!(status.is_empty(), "{status}");
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, "Hidden from git 1"), "the repository is read again");
}

#[test]
fn changes_unskip__show_its_changes_to_git_again_lists_the_file_in_the_changes_as_modified() {
    let repository = hidden();
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let _ = name_of(&tree, "config/local.json");
    let _ = choose(
        &driver,
        &mut state,
        &mut window,
        (words::SHOW_TO_GIT, "config/local.json"),
    );
    until_ended(&driver, &mut state);
    let status = repository.git(&["status", "--porcelain=v1"]);
    assert!(status.contains(" M config/local.json"), "{status}");
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::CheckBox, "config/local.json"));
    assert!(!says(&tree, "Hidden from git 1"));
}

#[test]
fn changes_skipped_list__the_group_matches_its_snapshots_in_each_theme() {
    let repository = hidden();
    for (theme, name) in [(ThemeName::Dark, "dark"), (ThemeName::Light, "light")] {
        let (state, _driver) = changes_page(&repository);
        let mut window = Window::new(theme, (900, 520));
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(&format!("changes-hidden-from-git-{name}"));
    }
}

#[test]
fn changes_skipped_list__the_files_page_marks_a_hidden_file_and_offers_to_show_it_to_git_again() {
    let repository = hidden();
    let (state, mut window, _driver) = files_page(&repository, &["config"]);
    let intents = super::files_page::choose(&mut window, &state, ("local.json", words::SHOW_TO_GIT));
    assert!(
        !intents.is_empty(),
        "the menu of a hidden file has Show its changes to git again"
    );
    let (state, mut window, _driver) = files_page(&repository, &["config"]);
    let _ = window.draw(&state, &[]);
    window.assert_snapshot("files-hidden-from-git-dark");
}

#[test]
fn changes_skipped_list__in_a_sparse_checkout_the_files_it_leaves_out_are_not_listed() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the folders", &[("in/a.txt", "a\n"), ("out/b.txt", "b\n")]);
    let _ = repository.git(&["sparse-checkout", "set", "in"]);
    let (state, _driver) = changes_page(&repository);
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    assert!(!says(&tree, "Hidden from git 1"));
    assert!(!has(&tree, Role::Button, "out/b.txt"));
}
