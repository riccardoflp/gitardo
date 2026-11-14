//! The files of the inspector drawn from the store, from repositories made for each test: cut at 300 with **Show
//! all**, laid out as a tree whose directories fold, the files of a stash and the untracked files it took, and the
//! pictures of the inspector in each theme.

use app::store::InspectorRead;
use platform::accesskit::Role;
use render::{Rect, ThemeName};
use testkit::Repository;
use views::Intent;
use views::details::{CappedList, DetailsIntent, FileLayout, FilesIntent, FilesOwner};

use super::inspector::{TALL, WIDE, ask, drawn, repository_for_details, select};
use super::showing;
use super::support::{Window, find, says, says_in};

/// A repository whose last commit adds so many files, in two directories, and whose commit before adds one.
fn repository_of_files(count: usize) -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let names: Vec<String> = (0..count)
        .map(|index| format!("{}/file{index:03}.txt", if index % 2 == 0 { "even" } else { "odd" }))
        .collect();
    let files: Vec<(&str, &str)> = names.iter().map(|name| (name.as_str(), "a line\n")).collect();
    let _ = repository.commit("Add many files", &files);
    repository
}

/// The count of the files of a tree.
fn files_in(tree: &[model::details::TreeNode]) -> usize {
    tree.iter()
        .map(|node| match node {
            model::details::TreeNode::Directory { children, .. } => files_in(children),
            model::details::TreeNode::File { .. } => 1,
        })
        .sum()
}

#[test]
fn details_show_all_files__a_list_of_301_files_shows_300_and_show_all_301_files() {
    let capped = CappedList {
        owner: FilesOwner::Inspector,
        count: 301,
        is_all_shown: false,
    };
    assert_eq!(capped.shown(), 300);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (intents, tree) = window.frame(&[], |ui| {
        ui.clip(Rect::new(0.0, 0.0, 300.0, 40.0), |ui| {
            views::details::show_all_files(&capped, ui)
        })
    });
    assert_eq!(intents, [] as [Intent; 0]);
    let _link = find(&tree, Role::Button, "Show all 301 files\u{2026}");
    let repository = repository_of_files(301);
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Add many files");
    ask(
        &mut state,
        Intent::Details(DetailsIntent::Files(FilesIntent::Layout(FileLayout::Tree))),
    );
    assert_eq!(files_in(&state.inspector.tree), 300);
}

#[test]
fn details_show_all_files__a_list_of_300_files_shows_them_all_with_no_link() {
    let capped = CappedList {
        owner: FilesOwner::Inspector,
        count: 300,
        is_all_shown: false,
    };
    assert_eq!(capped.shown(), 300);
    assert!(!capped.is_cut());
    let repository = repository_of_files(300);
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Add many files");
    assert_eq!(files_in(&state.inspector.tree), 300);
}

#[test]
fn details_show_all_files__show_all_shows_every_file_and_another_commit_starts_again_from_300() {
    let repository = repository_of_files(301);
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Add many files");
    let capped = CappedList {
        owner: FilesOwner::Inspector,
        count: 301,
        is_all_shown: false,
    };
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let link = "Show all 301 files\u{2026}";
    let (_, tree) = window.frame(&[], |ui| {
        ui.clip(Rect::new(0.0, 0.0, 300.0, 40.0), |ui| {
            views::details::show_all_files(&capped, ui)
        })
    });
    let bounds = find(&tree, Role::Button, link).bounds().unwrap();
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a window is far smaller than an f32 counts"
    )]
    let point = render::Point::new(
        f64::midpoint(bounds.x0, bounds.x1) as f32,
        f64::midpoint(bounds.y0, bounds.y1) as f32,
    );
    let press = ui::PointerButtonEvent {
        button: ui::PointerButton::Primary,
        position: point,
        modifiers: ui::Modifiers::NONE,
        click_count: 1,
    };
    let events = [
        ui::InputEvent::PointerMoved(ui::PointerMove {
            position: point,
            modifiers: ui::Modifiers::NONE,
        }),
        ui::InputEvent::PointerPressed(press),
        ui::InputEvent::PointerReleased(press),
    ];
    let (intents, _) = window.frame(&events, |ui| {
        ui.clip(Rect::new(0.0, 0.0, 300.0, 40.0), |ui| {
            views::details::show_all_files(&capped, ui)
        })
    });
    assert_eq!(intents, [Intent::Details(DetailsIntent::Files(FilesIntent::ShowAll))]);
    ask(&mut state, intents[0].clone());
    assert!(state.inspector.is_all_files_shown);
    assert_eq!(files_in(&state.inspector.tree), 301);
    let _ = select(&mut state, "Add the readme");
    let _ = select(&mut state, "Add many files");
    assert!(!state.inspector.is_all_files_shown);
    assert_eq!(files_in(&state.inspector.tree), 300);
}

#[test]
fn details_show_all_files__the_count_and_the_buttons_of_the_header_act_on_the_whole_list() {
    let repository = repository_of_files(301);
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Add many files");
    let tree = drawn(&state);
    assert!(says(&tree, "301 changed files"), "the count is of every file");
    let _layout = find(&tree, Role::Button, "Under their directories");
    ask(
        &mut state,
        Intent::Details(DetailsIntent::Files(FilesIntent::Layout(FileLayout::Tree))),
    );
    assert_eq!(state.persisted.file_layouts.inspector, FileLayout::Tree);
    assert!(says(&drawn(&state), "301 changed files"));
}

#[test]
fn details_show_all_files__in_the_tree_layout_the_tree_is_made_of_the_files_shown() {
    let repository = repository_of_files(301);
    let mut state = showing(repository.path().to_path_buf());
    ask(
        &mut state,
        Intent::Details(DetailsIntent::Files(FilesIntent::Layout(FileLayout::Tree))),
    );
    let _ = select(&mut state, "Add many files");
    let InspectorRead::Read(read) = &state.inspector.read else {
        panic!("the details are read");
    };
    let shown: Vec<&str> = read
        .detail
        .files
        .iter()
        .take(300)
        .map(|file| file.path.as_str())
        .collect();
    let mut in_tree = Vec::new();
    paths_in(&state.inspector.tree, &mut in_tree);
    in_tree.sort_unstable();
    let mut expected: Vec<String> = shown.iter().map(|path| (*path).to_owned()).collect();
    expected.sort_unstable();
    assert_eq!(in_tree, expected);
}

/// The paths of the files of a tree.
fn paths_in(tree: &[model::details::TreeNode], paths: &mut Vec<String>) {
    for node in tree {
        match node {
            model::details::TreeNode::Directory { children, .. } => paths_in(children, paths),
            model::details::TreeNode::File { path, .. } => paths.push(path.as_str().to_owned()),
        }
    }
}

#[test]
fn details_file_tree__folding_a_directory_hides_its_files_and_the_fold_is_forgotten_for_another_commit() {
    let repository = repository_for_details();
    let mut state = showing(repository.path().to_path_buf());
    ask(
        &mut state,
        Intent::Details(DetailsIntent::Files(FilesIntent::Layout(FileLayout::Tree))),
    );
    let _ = select(&mut state, "Explain the timeline");
    assert!(says(&drawn(&state), "main.rs"));
    let intents = Window::new(ThemeName::Dark, TALL).click(&state, Role::Button, "src/app");
    assert_eq!(
        intents,
        [Intent::Details(DetailsIntent::Files(FilesIntent::Fold(String::from(
            "src/app"
        ))))]
    );
    ask(&mut state, intents[0].clone());
    let tree = drawn(&state);
    assert!(!says(&tree, "main.rs"), "the file of the directory folded is hidden");
    assert!(says(&tree, "guide.md"), "the others stay");
    ask(&mut state, intents[0].clone());
    assert!(says(&drawn(&state), "main.rs"), "unfolded");
    ask(&mut state, intents[0].clone());
    let _ = select(&mut state, "Change the readme");
    let _ = select(&mut state, "Explain the timeline");
    assert!(state.inspector.folded.is_empty());
    assert!(says(&drawn(&state), "main.rs"));
}

/// A repository with a stash made with an untracked file, and a stash made without.
fn repository_with_stashes() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    std::fs::write(repository.path().join("README.md"), "Gitardo, changed\n").unwrap();
    let _ = repository.git(&["stash", "push", "--quiet", "--message", "only the readme"]);
    std::fs::write(repository.path().join("README.md"), "Gitardo, again\n").unwrap();
    std::fs::write(repository.path().join("notes.txt"), "new notes\n").unwrap();
    let _ = repository.git(&[
        "stash",
        "push",
        "--quiet",
        "--include-untracked",
        "--message",
        "with the notes",
    ]);
    repository
}

#[test]
fn details_stash__a_stash_shows_its_message_author_date_and_hash_and_the_files_it_changed() {
    let repository = repository_with_stashes();
    let mut state = showing(repository.path().to_path_buf());
    let stash = select(&mut state, "On main: only the readme");
    let tree = drawn(&state);
    for text in [
        "On main: only the readme",
        "Test Author",
        "Date",
        "1 changed file",
        "README.md",
    ] {
        assert!(says(&tree, text), "the stash shows {text}");
    }
    let _hash = find(&tree, Role::Button, stash.short());
}

#[test]
fn details_stash__a_stash_made_with_untracked_files_shows_them_under_their_count_apart() {
    let repository = repository_with_stashes();
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "On main: with the notes");
    let tree = drawn(&state);
    assert!(says(&tree, "1 changed file"), "the readme");
    assert!(says(&tree, "1 untracked file"));
    assert!(says(&tree, "notes.txt"));
}

#[test]
fn details_stash__a_click_on_an_untracked_file_opens_its_diff_page_against_nothing_all_its_lines_added() {
    let repository = repository_with_stashes();
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "On main: with the notes");
    let mut window = Window::new(ThemeName::Dark, TALL);
    let (_, tree) = window.draw(&state, &[]);
    let point = super::diff_links::words_at(&tree, "notes.txt");
    let intents = super::pointer::click_at(&mut window, &state, point, 1);
    let targets = super::diff_links::opened(&intents);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].left, views::diff::DiffVersion::Empty);
    super::pointer::apply(&mut state, intents);
    let read = super::diff::text(super::diff::shown(&state));
    assert_eq!(read.aligned.rows.len(), 1);
    assert!(read.aligned.rows.iter().all(|row| row.left.is_none()));
    assert_eq!(read.right_line(0), "new notes");
}

#[test]
fn details_stash__a_stash_made_without_untracked_files_shows_no_such_section() {
    let repository = repository_with_stashes();
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "On main: only the readme");
    assert!(!says_in(&drawn(&state), "untracked file"));
}

#[test]
fn details_stash__the_page_of_a_stash_shows_the_untracked_files_the_same_way() {
    let repository = repository_with_stashes();
    let mut state = showing(repository.path().to_path_buf());
    let stash = select(&mut state, "On main: with the notes");
    ask(&mut state, Intent::Details(DetailsIntent::OpenPage(stash)));
    let tree = drawn(&state);
    assert!(says(&tree, "1 untracked file"));
    assert!(says(&tree, "notes.txt"));
}

#[test]
fn the_inspector_with_a_commit_its_message_and_its_files_as_a_tree_is_drawn_in_each_theme() {
    let repository = repository_for_details();
    let mut state = showing(repository.path().to_path_buf());
    ask(
        &mut state,
        Intent::Details(DetailsIntent::Files(FilesIntent::Layout(FileLayout::Tree))),
    );
    let _ = select(&mut state, "Explain the timeline");
    for (theme, name) in [
        (ThemeName::Dark, "inspector-commit-dark"),
        (ThemeName::Light, "inspector-commit-light"),
    ] {
        let mut window = Window::new(theme, WIDE);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(name);
    }
}

#[test]
fn the_inspector_with_a_stash_and_its_untracked_files_is_drawn() {
    let repository = repository_with_stashes();
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "On main: with the notes");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = window.draw(&state, &[]);
    window.assert_snapshot("inspector-stash-dark");
}
