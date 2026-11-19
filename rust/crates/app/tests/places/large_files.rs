//! The Large files page (`lfs.page`) on repositories made for each test with the Git LFS installed on the machine and
//! a remote beside them, never a server: the patterns and the files with **Download all**, which pulls them; the head
//! without Git LFS; the menus of the patterns; **Track a pattern…**; and the snapshots with files and with none.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use app::store::{Message, State};
use app::work::large_files::LargeFilesRead;
use model::contract::Answer;
use model::lfs::{LfsFile, LfsPattern, LfsStatus};
use platform::accesskit::{Role, TreeUpdate};
use render::{Point, ThemeName};
use testkit::{Remote, Repository};
use views::Intent;
use views::feature::FeatureIntent;
use views::lfs::LargeFilesIntent;
use views::text::lfs as words;

use super::changes_menu::right_click;
use super::lfs::{git_in, stored};
use super::markdown::drawn;
use super::showing;
use super::side_column::menu_items;
use super::support::{OperationsDriver, Window, has, run, says};

/// The size of the window of the page.
const SIZE: (u32, u32) = (1000, 600);
/// The size of `art/logo.psd`: 2.0 MB.
const LOGO_SIZE: usize = 2 * 1024 * 1024;

/// A text of so many bytes, of lines of one letter.
fn content(size: usize, letter: char) -> String {
    let line: String = std::iter::repeat_n(letter, 63).chain(['\n']).collect();
    let mut text = line.repeat(size / 64 + 1);
    text.truncate(size);
    text
}

/// A repository storing `art/logo.psd` of 2.0 MB and `art/cover.psd` of 1.1 KB with Git LFS, pushed to a remote, and a
/// clone of it made without the contents in which only `art/cover.psd` was pulled.
fn one_not_downloaded() -> (Repository, PathBuf) {
    let repository = stored(&[
        ("art/logo.psd", &content(LOGO_SIZE, 'a')),
        ("art/cover.psd", &content(1200, 'c')),
    ]);
    let remote = Remote::new(&repository);
    let clone = repository.path().parent().unwrap().join("clone");
    let output = repository
        .command("git")
        .current_dir(repository.path().parent().unwrap())
        .env("GIT_LFS_SKIP_SMUDGE", "1")
        .args(["clone", "--quiet"])
        .arg(remote.path())
        .arg(&clone)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let _ = git_in(&repository, &clone, &["lfs", "install", "--local"]);
    for (key, value) in [("user.name", "Test Author"), ("user.email", "author@example.com")] {
        let _ = git_in(&repository, &clone, &["config", key, value]);
    }
    let _ = git_in(&repository, &clone, &["lfs", "pull", "--include=art/cover.psd"]);
    (repository, clone)
}

/// A window showing a folder with the Large files page opened, drawn once.
fn page(folder: &Path, theme: ThemeName) -> (State, Window) {
    let mut state = showing(folder.to_path_buf());
    let _ = run(&mut state, Message::Intent(Intent::LargeFiles(LargeFilesIntent::Open)));
    let mut window = Window::new(theme, SIZE);
    let _ = window.draw(&state, &[]);
    (state, window)
}

/// The middle of the lowest text that reads so.
fn point_of(tree: &TreeUpdate, text: &str) -> Point {
    let found = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.label() == Some(text) || node.value() == Some(text))
        .filter_map(|(_, node)| node.bounds().map(super::pointer::rect))
        .max_by(|one, other| one.y.total_cmp(&other.y))
        .unwrap_or_else(|| panic!("the text {text}"));
    Point::new(found.x + found.width * 0.5, found.y + found.height * 0.5)
}

/// The items of the menu of the row of a pattern or a file, opened by the button of context menus.
fn menu_of(window: &mut Window, state: &State, text: &str) -> Vec<String> {
    let (_, tree) = window.draw(state, &[]);
    menu_items(&right_click(window, state, point_of(&tree, text)))
}

/// Clicks a button, answers the questions of the operation it runs with these answers in order, confirms its plan,
/// and gives the lines it logged.
fn run_answering(window: &mut Window, state: &mut State, name: &str, answers: &[Answer]) -> Vec<String> {
    let driver = OperationsDriver::new();
    let mut answers: VecDeque<Answer> = answers.iter().cloned().collect();
    for intent in window.click(state, Role::Button, name) {
        driver.intent(state, intent);
    }
    loop {
        match driver.next(state) {
            Message::FeatureAsks(question) => {
                let answer = answers
                    .pop_front()
                    .unwrap_or_else(|| panic!("an answer to {question:?}"));
                driver.intent(state, Intent::Feature(FeatureIntent::Answer(Some(answer))));
            }
            Message::FeatureConfirms(_) => driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureEnded { ended, log } => {
                assert!(matches!(ended, app::store::feature::Ended::Ran { .. }), "{ended:?}");
                return log.into_iter().map(|entry| entry.line).collect();
            }
            _ => {}
        }
    }
}

#[test]
fn lfs_page__with_psd_tracked_and_two_files_one_not_downloaded_it_lists_them_and_download_all_1_2_0_mb() {
    let (_repository, clone) = one_not_downloaded();
    let (state, mut window) = page(&clone, ThemeName::Dark);
    let tree = drawn(&mut window, &state);
    assert!(says(&tree, "*.psd"));
    assert!(says(&tree, "art/logo.psd"));
    assert!(says(&tree, "art/cover.psd"));
    assert!(says(&tree, "2.0 MB"));
    assert!(says(&tree, "1.1 KB"));
    assert!(has(&tree, Role::Button, "Download all (1, 2.0 MB)"));
    assert!(says(&tree, words::NOT_DOWNLOADED));
    assert!(says(&tree, words::DOWNLOADED));
}

#[test]
fn lfs_download__download_all_on_the_page_pulls_every_file_and_the_page_lists_them_downloaded_after() {
    let (_repository, clone) = one_not_downloaded();
    let (mut state, mut window) = page(&clone, ThemeName::Dark);
    let log = run_answering(&mut window, &mut state, "Download all (1, 2.0 MB)", &[]);
    assert_eq!(log, ["git lfs pull"]);
    let tree = drawn(&mut window, &state);
    assert!(!says(&tree, words::NOT_DOWNLOADED));
    assert!(has(&tree, Role::Button, "Download all"));
    let on_disk = std::fs::read_to_string(clone.join("art/logo.psd")).unwrap();
    assert!(on_disk.starts_with("aaaa"));
}

#[test]
fn lfs_page__without_git_lfs_the_head_says_it_is_not_installed_and_the_patterns_are_listed_all_the_same() {
    let repository = stored(&[]);
    let (mut state, mut window) = page(repository.path(), ThemeName::Dark);
    // Git LFS cannot be taken away from the machine in a test: the page is given what a read without it gives.
    state.large_files.read = Some(Ok(LargeFilesRead {
        status: LfsStatus {
            version: None,
            is_set_up: false,
            patterns: vec![LfsPattern {
                pattern: String::from("*.psd"),
                file: String::from(".gitattributes"),
                line: 1,
            }],
        },
        files: Vec::new(),
    }));
    let tree = drawn(&mut window, &state);
    assert!(says(&tree, words::NOT_INSTALLED));
    assert!(has(&tree, Role::Button, words::OPEN_SITE));
    assert!(says(&tree, "*.psd"));
    assert!(!says(&tree, words::NOT_SET_UP));
    let intents = window.click(&state, Role::Button, words::OPEN_SITE);
    assert_eq!(intents, [Intent::OpenUrl(String::from("https://git-lfs.com"))]);
}

#[test]
fn lfs_untrack__a_pattern_of_assets_gitattributes_has_open_assets_gitattributes_and_no_stop_storing() {
    let mut repository = stored(&[]);
    let _ = repository.commit(
        "Store the images of the assets",
        &[("assets/.gitattributes", "*.png filter=lfs diff=lfs merge=lfs -text\n")],
    );
    let (state, mut window) = page(repository.path(), ThemeName::Dark);
    let of_assets = menu_of(&mut window, &state, "*.png");
    assert!(
        of_assets.contains(&String::from("Open assets/.gitattributes")),
        "{of_assets:?}"
    );
    assert!(!of_assets.contains(&String::from(words::STOP_STORING)), "{of_assets:?}");
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let of_top = menu_of(&mut window, &state, "*.psd");
    assert!(of_top.contains(&String::from(words::STOP_STORING)), "{of_top:?}");
}

#[test]
fn lfs_untrack__stop_storing_on_star_psd_takes_the_line_out_of_gitattributes() {
    let repository = stored(&[]);
    let (mut state, mut window) = page(repository.path(), ThemeName::Dark);
    let items = menu_of(&mut window, &state, "*.psd");
    assert!(items.contains(&String::from(words::STOP_STORING)));
    let driver = OperationsDriver::new();
    let intents = window.click(&state, Role::MenuItem, words::STOP_STORING);
    for intent in intents {
        driver.intent(&mut state, intent);
    }
    let log = loop {
        match driver.next(&mut state) {
            Message::FeatureConfirms(_) => driver.intent(&mut state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureEnded { log, .. } => break log,
            _ => {}
        }
    };
    let lines: Vec<String> = log.into_iter().map(|entry| entry.line).collect();
    assert_eq!(lines, ["git lfs untrack *.psd"]);
    let attributes = std::fs::read_to_string(repository.path().join(".gitattributes")).unwrap();
    assert!(!attributes.contains("*.psd"), "{attributes}");
}

#[test]
fn lfs_track__track_a_pattern_on_the_page_runs_git_lfs_track_and_gitattributes_has_the_line() {
    let repository = stored(&[]);
    let (mut state, mut window) = page(repository.path(), ThemeName::Dark);
    let log = run_answering(
        &mut window,
        &mut state,
        words::TRACK,
        &[Answer::Text(String::from("*.tif"))],
    );
    assert_eq!(log, ["git lfs track *.tif"]);
    let attributes = std::fs::read_to_string(repository.path().join(".gitattributes")).unwrap();
    assert!(
        attributes.contains("*.tif filter=lfs diff=lfs merge=lfs -text"),
        "{attributes}"
    );
    let tree = drawn(&mut window, &state);
    assert!(says(&tree, "*.tif"));
}

#[test]
fn lfs_page__the_page_matches_its_snapshots_in_each_theme_with_files_and_with_none() {
    let (_repository, clone) = one_not_downloaded();
    let nothing = Repository::new();
    for (folder, kind) in [(clone.as_path(), "files"), (nothing.path(), "none")] {
        for (theme, name) in [(ThemeName::Dark, "dark"), (ThemeName::Light, "light")] {
            let (mut state, mut window) = page(folder, theme);
            // The version is the one of the machine: the snapshot is drawn with one of its own.
            if let Some(Ok(read)) = &mut state.large_files.read {
                read.status.version = Some(String::from("3.7.1"));
                read.files
                    .sort_by(|one: &LfsFile, other: &LfsFile| one.path.cmp(&other.path));
            }
            let _ = window.draw(&state, &[]);
            window.assert_snapshot(&format!("lfs-page-{kind}-{name}"));
        }
    }
}

#[test]
fn repo_tabs__large_files_in_the_menu_of_the_row_of_a_repository_opens_the_large_files_page() {
    let repository = stored(&[]);
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let items = menu_of(&mut window, &state, "repository");
    assert!(items.contains(&String::from(words::MENU_ENTRY)), "{items:?}");
    for intent in window.click(&state, Role::MenuItem, words::MENU_ENTRY) {
        let _ = run(&mut state, Message::Intent(intent));
    }
    assert_eq!(state.page_shown(), Some(views::page::Page::LargeFiles));
    let tree = drawn(&mut window, &state);
    assert!(says(&tree, "*.psd"));
    assert!(views::search::QuickCommand::LargeFiles.is_built());
}
