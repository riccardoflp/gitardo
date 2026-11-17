//! The tree of the Files page marking the files that differ from the last commit, as the Changes page marks them, on
//! repositories made for each test (`file-view.files-changes`).

use std::time::Duration;

use app::store::{Message, State};
use model::changes::{FolderMark, TreeMark};
use model::state::ChangeKind;
use platform::accesskit::TreeUpdate;
use render::ThemeName;
use testkit::{CONFLICT_FILE, Repository};
use ui::{Appearance, Theme};
use views::Intent;
use views::page::Page;

use super::files_page::{files_page, row};
use super::pointer::{click_at, moved};
use super::support::{self, Window, says};

/// A repository with `README.md`, `src/app.rs` and `src/ui/list.rs` committed.
fn committed() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit(
        "Write the app",
        &[
            ("README.md", "readme\n"),
            ("src/app.rs", "app\n"),
            ("src/ui/list.rs", "list\n"),
        ],
    );
    repository
}

/// Writes a file of a repository.
fn write(repository: &Repository, path: &str, text: &str) {
    let file = repository.path().join(path);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(file, text).unwrap();
}

/// Reads the repository again, as the watcher would once a file changed.
fn read_again(state: &mut State) {
    let _effects = support::run(state, Message::Intent(Intent::Refresh));
}

/// How a file of the tree is marked, by its path.
fn mark(state: &State, path: &str) -> Option<TreeMark> {
    state.files_page.marks.files.get(path).map(|file| file.mark)
}

/// What the window says, drawn.
fn drawn(window: &mut Window, state: &State) -> TreeUpdate {
    window.draw(state, &[]).1
}

#[test]
fn file_view_files_changes__a_modified_file_is_marked_m_and_the_screen_reader_hears_app_rs_modified() {
    let repository = committed();
    write(&repository, "src/app.rs", "app, changed\n");
    let (state, mut window, _driver) = files_page(&repository, &["src"]);
    let tree = drawn(&mut window, &state);
    assert!(says(&tree, "app.rs, modified"));
    assert_eq!(
        mark(&state, "src/app.rs"),
        Some(TreeMark::Changed(ChangeKind::Modified))
    );
    assert!(
        says(&tree, "README.md"),
        "a file as committed is said by its name alone"
    );
    assert_eq!(mark(&state, "README.md"), None);
}

#[test]
fn file_view_files_changes__a_new_file_not_tracked_is_marked_untracked_and_one_staged_added() {
    let repository = committed();
    write(&repository, "notes.txt", "notes\n");
    write(&repository, "todo.txt", "todo\n");
    let _ = repository.git(&["add", "todo.txt"]);
    let (state, mut window, _driver) = files_page(&repository, &[]);
    let tree = drawn(&mut window, &state);
    assert!(says(&tree, "notes.txt, untracked"));
    assert!(says(&tree, "todo.txt, added"));
    assert_eq!(mark(&state, "notes.txt"), Some(TreeMark::Untracked));
    assert_eq!(mark(&state, "todo.txt"), Some(TreeMark::Changed(ChangeKind::Added)));
}

#[test]
fn file_view_files_changes__a_file_deleted_from_disk_and_one_whose_deletion_is_staged_are_both_at_their_places() {
    let mut repository = committed();
    let _ = repository.commit("Add two files", &[("src/gone.rs", "gone\n"), ("src/old.rs", "old\n")]);
    std::fs::remove_file(repository.path().join("src/gone.rs")).unwrap();
    let _ = repository.git(&["rm", "--quiet", "src/old.rs"]);
    let (state, mut window, _driver) = files_page(&repository, &["src"]);
    let tree = drawn(&mut window, &state);
    let (app, gone, old) = (
        row(&tree, "app.rs"),
        row(&tree, "gone.rs, deleted"),
        row(&tree, "old.rs, deleted"),
    );
    assert!(
        app.y < gone.y && gone.y < old.y,
        "the files deleted are in the order of their names"
    );
    assert_eq!(
        mark(&state, "src/gone.rs"),
        Some(TreeMark::Changed(ChangeKind::Deleted))
    );
    assert_eq!(mark(&state, "src/old.rs"), Some(TreeMark::Changed(ChangeKind::Deleted)));
}

#[test]
fn file_view_files_changes__a_click_on_a_deleted_file_opens_its_diff_page_not_a_file_page() {
    let repository = committed();
    let _ = repository.git(&["rm", "--quiet", "README.md"]);
    let (mut state, mut window, _driver) = files_page(&repository, &[]);
    let point = row(&drawn(&mut window, &state), "README.md, deleted");
    for intent in click_at(&mut window, &state, point, 1) {
        let _effects = support::run(&mut state, Message::Intent(intent));
    }
    assert!(
        matches!(state.page_shown(), Some(Page::Diff(_))),
        "{:?}",
        state.page_shown()
    );
    assert_eq!(state.files_page.shown_file, None);
}

#[test]
fn file_view_files_changes__a_staged_rename_is_shown_at_its_new_path_its_tooltip_naming_the_old_one() {
    let repository = committed();
    let _ = repository.git(&["mv", "src/app.rs", "src/main.rs"]);
    let (state, mut window, _driver) = files_page(&repository, &["src"]);
    let tree = drawn(&mut window, &state);
    assert!(says(&tree, "main.rs, renamed from src/app.rs"));
    assert!(
        !says(&tree, "app.rs") && !says(&tree, "app.rs, deleted"),
        "the old name is not listed"
    );
    let renamed = state.files_page.marks.files.get("src/main.rs");
    assert_eq!(
        renamed.and_then(|file| file.original_path.as_deref()),
        Some("src/app.rs")
    );
    let before = tooltip_pixels(&window);
    let _ = window.draw(&state, &[moved(row(&tree, "main.rs, renamed from src/app.rs"))]);
    window.now += Duration::from_secs(1);
    let _ = window.draw(&state, &[]);
    assert!(
        tooltip_pixels(&window) > before + 200,
        "the tooltip naming the old name is drawn"
    );
}

#[test]
fn file_view_files_changes__a_file_in_conflict_is_marked_in_conflict_over_anything_else() {
    let mut repository = committed();
    repository.conflict();
    let (state, mut window, _driver) = files_page(&repository, &[]);
    assert!(says(
        &drawn(&mut window, &state),
        &format!("{CONFLICT_FILE}, in conflict")
    ));
    assert_eq!(mark(&state, CONFLICT_FILE), Some(TreeMark::Conflicted));
}

#[test]
fn file_view_files_changes__a_folded_folder_holding_a_change_has_a_dot_of_its_most_important_state() {
    let repository = committed();
    write(&repository, "src/ui/list.rs", "list, changed\n");
    write(&repository, "src/new.rs", "new\n");
    let (mut state, mut window, _driver) = files_page(&repository, &[]);
    assert!(says(&drawn(&mut window, &state), "src, has changes"));
    assert!(
        says(&drawn(&mut window, &state), "README.md"),
        "a file as committed is said by its name alone"
    );
    assert_eq!(state.files_page.marks.folders.get("src"), Some(&FolderMark::Modified));
    assert!(state.files_page.opened.is_empty(), "the folder stays folded");
    std::fs::remove_file(repository.path().join("src/app.rs")).unwrap();
    read_again(&mut state);
    assert_eq!(state.files_page.marks.folders.get("src"), Some(&FolderMark::Deleted));
    assert_eq!(
        state.files_page.marks.folders.get("src/ui"),
        Some(&FolderMark::Modified)
    );
}

#[test]
fn file_view_files_changes__the_marks_follow_each_read_without_folding_or_unfolding_a_folder() {
    let repository = committed();
    let (mut state, mut window, _driver) = files_page(&repository, &["src"]);
    assert!(says(&drawn(&mut window, &state), "app.rs"));
    write(&repository, "src/app.rs", "app, changed\n");
    read_again(&mut state);
    assert!(says(&drawn(&mut window, &state), "app.rs, modified"));
    assert_eq!(state.files_page.opened.iter().collect::<Vec<_>>(), ["src"]);
    write(&repository, "src/app.rs", "app\n");
    read_again(&mut state);
    assert!(says(&drawn(&mut window, &state), "app.rs"));
    assert_eq!(mark(&state, "src/app.rs"), None);
    assert_eq!(state.files_page.opened.iter().collect::<Vec<_>>(), ["src"]);
}

#[test]
fn the_tree_of_the_files_page_with_its_marks_is_drawn_in_each_theme() {
    let mut repository = committed();
    let _ = repository.commit("Add the old files", &[("gone.txt", "gone\n"), ("src/old.rs", "old\n")]);
    repository.conflict();
    write(&repository, "src/app.rs", "app, changed\n");
    write(&repository, "src/ui/list.rs", "list, changed\n");
    write(&repository, "notes.txt", "notes\n");
    write(&repository, "todo.txt", "todo\n");
    let _ = repository.git(&["add", "todo.txt"]);
    std::fs::remove_file(repository.path().join("gone.txt")).unwrap();
    let _ = repository.git(&["mv", "src/old.rs", "src/renamed.rs"]);
    for (theme, name) in [
        (ThemeName::Dark, "files-changes-dark"),
        (ThemeName::Light, "files-changes-light"),
        (ThemeName::HighContrastDark, "files-changes-high-contrast-dark"),
        (ThemeName::HighContrastLight, "files-changes-high-contrast-light"),
    ] {
        let (state, _window, _driver) = files_page(&repository, &["src"]);
        let mut window = Window::new(theme, (1000, 700));
        let _ = window.draw(&state, &[]);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(name);
    }
}

/// How many pixels are of the background of tooltips.
fn tooltip_pixels(window: &Window) -> usize {
    let color = Theme::new(ThemeName::Dark, Appearance::Dark).colors.tooltip_background;
    window
        .frame
        .to_rgba()
        .chunks(4)
        .filter(|pixel| {
            pixel.first() == Some(&color.red) && pixel.get(1) == Some(&color.green) && pixel.get(2) == Some(&color.blue)
        })
        .count()
}
