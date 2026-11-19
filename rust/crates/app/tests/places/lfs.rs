//! Files stored with Git LFS in the pages, on repositories made for each test with the Git LFS installed on the
//! machine and a remote beside them, never a server: a file left as a pointer said not downloaded in the File page
//! with **Download**, which pulls it; the diff of the contents in the Diff page; the line of the sizes in the page of a
//! commit; a pointer at a path Git LFS does not store shown as text; and the snapshots of a file not downloaded.

use std::path::{Path, PathBuf};

use app::store::{FileContent, Message, State};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::{Remote, Repository};
use views::Intent;
use views::diff::{DiffFile, DiffTarget, DiffVersion, FileDiff};
use views::feature::FeatureIntent;
use views::file_view::FileTarget;
use views::text::lfs as words;

use super::diff::{head_to_working_tree, open as open_diff, shown as diff_shown, text as diff_text};
use super::file_view::{open, shown};
use super::markdown::drawn;
use super::showing;
use super::support::{OperationsDriver, Window, has, says, says_in};

/// The size of the window of the pages.
const SIZE: (u32, u32) = (1000, 600);
/// The size of the first content of `art/logo.psd`: 2.0 MB.
const FIRST_SIZE: usize = 2 * 1024 * 1024;
/// The size of its second content: 2.1 MB.
const SECOND_SIZE: usize = 2_202_010;

/// A text of so many bytes, of lines of one letter.
fn content(size: usize, letter: char) -> String {
    let line: String = std::iter::repeat_n(letter, 63).chain(['\n']).collect();
    let mut text = line.repeat(size / 64 + 1);
    text.truncate(size);
    text
}

/// Runs git in a folder in the environment of the repository's tests, and gives what it printed.
pub(super) fn git_in(repository: &Repository, folder: &Path, arguments: &[&str]) -> String {
    let output = repository
        .command("git")
        .current_dir(folder)
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// A repository set up for Git LFS that stores `*.psd` with it, with a first commit of the attributes and the files.
pub(super) fn stored(files: &[(&str, &str)]) -> Repository {
    let mut repository = Repository::new();
    let _ = repository.git(&["lfs", "install", "--local"]);
    let mut all = vec![(".gitattributes", "*.psd filter=lfs diff=lfs merge=lfs -text\n")];
    all.extend_from_slice(files);
    let _ = repository.commit("Add the files", &all);
    repository
}

/// A repository whose `art/logo.psd` of 2.0 MB was committed, then changed to 2.1 MB, pushed to `origin`; and a clone
/// of it made with `GIT_LFS_SKIP_SMUDGE=1`, set up for Git LFS, whose `art/logo.psd` is its pointer.
pub(super) fn cloned_without_contents() -> (Repository, PathBuf) {
    let mut repository = stored(&[("art/logo.psd", &content(FIRST_SIZE, 'a'))]);
    let _ = repository.commit("Change the logo", &[("art/logo.psd", &content(SECOND_SIZE, 'b'))]);
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
    // Commits of the clone are made as the repository's are, never as the person running the tests.
    for (key, value) in [("user.name", "Test Author"), ("user.email", "author@example.com")] {
        let _ = git_in(&repository, &clone, &["config", key, value]);
    }
    (repository, clone)
}

/// A window showing a folder, with the File page of a file of its working tree, drawn once.
fn file_page(folder: &Path, path: &str, theme: ThemeName) -> (State, Window) {
    let mut state = showing(folder.to_path_buf());
    open(&mut state, FileTarget::working_tree(path));
    let mut window = Window::new(theme, SIZE);
    let _ = window.draw(&state, &[]);
    (state, window)
}

/// Draws the window, runs the operation the click on a button asks through the operations thread, and gives the lines
/// it logged.
pub(super) fn run_clicked(window: &mut Window, state: &mut State, name: &str) -> Vec<String> {
    let driver = OperationsDriver::new();
    for intent in window.click(state, Role::Button, name) {
        driver.intent(state, intent);
    }
    assert!(state.operations.running.is_some(), "{name} runs an operation");
    loop {
        match driver.next(state) {
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
fn lfs_show_in_pages__after_a_clone_without_contents_art_logo_psd_says_not_downloaded_with_its_size_and_download() {
    let (_repository, clone) = cloned_without_contents();
    let (state, mut window) = file_page(&clone, "art/logo.psd", ThemeName::Dark);
    assert!(
        matches!(shown(&state).content, FileContent::StoredWithLfs(_)),
        "{:?}",
        shown(&state).content
    );
    let tree = drawn(&mut window, &state);
    assert!(says(&tree, words::NOT_DOWNLOADED_HEADING));
    assert!(says_in(&tree, "2.1 MB"));
    assert!(has(&tree, Role::Button, words::DOWNLOAD));
    assert!(!says_in(&tree, "version https://git-lfs.github.com"));
    assert!(!says_in(&tree, "oid sha256:"));
}

#[test]
fn lfs_download__download_on_art_logo_psd_pulls_it_and_the_file_page_shows_its_content_after() {
    let (_repository, clone) = cloned_without_contents();
    let (mut state, mut window) = file_page(&clone, "art/logo.psd", ThemeName::Dark);
    let log = run_clicked(&mut window, &mut state, words::DOWNLOAD);
    assert_eq!(log, ["git lfs pull --include=art/logo.psd"]);
    let FileContent::Text(document) = &shown(&state).content else {
        panic!("the content is shown, not {:?}", shown(&state).content);
    };
    assert!(document.buffer().text().starts_with("bbbb"));
}

#[test]
fn lfs_download__download_in_the_diff_page_of_a_commit_not_checked_out_fetches_it_and_leaves_the_working_tree() {
    let (repository, clone) = cloned_without_contents();
    let first = git_in(&repository, &clone, &["rev-parse", "HEAD~1"]);
    let first = model::CommitId::parse(first.trim()).unwrap();
    let mut state = showing(clone.clone());
    let target = DiffTarget::file(
        DiffVersion::Empty,
        DiffVersion::Commit {
            commit: first.clone(),
            name: first.short().to_owned(),
        },
        DiffFile::same("art/logo.psd"),
    );
    open_diff(&mut state, target);
    assert!(matches!(
        diff_shown(&state).diffs.first(),
        Some(FileDiff::StoredWithLfs(_))
    ));
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = drawn(&mut window, &state);
    assert!(says(&tree, words::STORED));
    let log = run_clicked(&mut window, &mut state, words::DOWNLOAD);
    assert_eq!(log, [format!("git lfs fetch --include=art/logo.psd origin {first}")]);
    let on_disk = std::fs::read_to_string(clone.join("art/logo.psd")).unwrap();
    assert!(
        on_disk.starts_with("version https://git-lfs.github.com/spec/v1"),
        "{on_disk}"
    );
    let text = diff_text(diff_shown(&state));
    assert!(text.is_stored_with_lfs);
    assert!(text.right.starts_with("aaaa"));
}

#[test]
fn lfs_show_in_pages__the_diff_page_of_a_text_file_stored_with_git_lfs_changed_and_downloaded_shows_its_lines() {
    let repository = stored(&[("notes.psd", "first line\nsecond line\n")]);
    std::fs::write(repository.path().join("notes.psd"), "first line\nsecond line changed\n").unwrap();
    let mut state = showing(repository.path().to_path_buf());
    open_diff(&mut state, head_to_working_tree("notes.psd"));
    let text = diff_text(diff_shown(&state));
    assert!(text.is_stored_with_lfs);
    assert_eq!(text.left, "first line\nsecond line\n");
    assert_eq!(text.right, "first line\nsecond line changed\n");
    assert_eq!(text.changes.len(), 1);
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = drawn(&mut window, &state);
    assert!(says(&tree, words::STORED));
}

#[test]
fn lfs_show_in_pages__the_page_of_a_commit_that_changed_art_logo_psd_says_its_two_sizes_in_place_of_the_pointers() {
    let (repository, _clone) = cloned_without_contents();
    let mut state = showing(repository.path().to_path_buf());
    let id = super::commit_page::open(&mut state, "Change the logo");
    super::commit_page::ask_page(&mut state, id, views::commit_page::CommitPageIntent::ShowChanges(true));
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = drawn(&mut window, &state);
    assert!(says(&tree, "Stored with Git LFS: 2.0 MB \u{2192} 2.1 MB"));
    assert!(has(&tree, Role::Button, words::OPEN_IN_DIFF_PAGE));
    assert!(!says_in(&tree, "oid sha256:"));
}

#[test]
fn lfs_show_in_pages__a_file_whose_text_is_a_pointer_at_a_path_git_lfs_does_not_store_is_shown_as_text() {
    let pointer = "version https://git-lfs.github.com/spec/v1\noid sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393\nsize 12\n";
    let repository = stored(&[("docs/pointer.txt", pointer)]);
    let (state, _window) = file_page(repository.path(), "docs/pointer.txt", ThemeName::Dark);
    assert_eq!(super::file_view::text(&state), pointer);
}

#[test]
fn lfs_show_in_pages__a_file_not_downloaded_matches_its_snapshot_in_each_theme() {
    let (_repository, clone) = cloned_without_contents();
    for (theme, name) in [
        (ThemeName::Dark, "lfs-not-downloaded-dark"),
        (ThemeName::Light, "lfs-not-downloaded-light"),
    ] {
        let (state, mut window) = file_page(&clone, "art/logo.psd", theme);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(name);
    }
}
