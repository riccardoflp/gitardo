//! The locks of the Large files page (`lfs.read-locks`, `lfs.lock`, `lfs.unlock`) on a clone whose `lfs.url` is a
//! server of the tests on 127.0.0.1, never a real one: read once when the page opens and never with a read of the
//! repository, the name of whoever locked a file with **Unlock** and its warning, and **Lock**.

use std::path::{Path, PathBuf};

use app::store::{Message, State};
use http::test_server::{Answer, TestServer, answer};
use model::contract::Danger;
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::feature::FeatureIntent;
use views::text::lfs as words;

use super::large_files::{menu_of, one_not_downloaded, page};
use super::lfs::git_in;
use super::markdown::drawn;
use super::support::{OperationsDriver, run, says};

/// The locks of a server with none.
const NO_LOCK: &str = r#"{"ours":[],"theirs":[]}"#;
/// The locks of a server on which `reviewer` locked `art/logo.psd`.
const THEIRS: &str = r#"{"ours":[],"theirs":[{"id":"7","path":"art/logo.psd","owner":{"name":"reviewer"},"locked_at":"2026-11-19T10:00:00Z"}]}"#;
/// The lock the server gives once `art/logo.psd` is locked by this user.
const LOCKED: &str =
    r#"{"lock":{"id":"8","path":"art/logo.psd","owner":{"name":"Test Author"},"locked_at":"2026-11-19T10:05:00Z"}}"#;
/// The locks of a server on which this user locked `art/logo.psd`.
const OURS: &str = r#"{"ours":[{"id":"8","path":"art/logo.psd","owner":{"name":"Test Author"},"locked_at":"2026-11-19T10:05:00Z"}],"theirs":[]}"#;

/// An answer of the API of Git LFS with this status and body.
fn lfs_answer(status: &str, body: &str) -> Answer {
    Answer::Bytes(answer(
        status,
        &[("Content-Type", "application/vnd.git-lfs+json")],
        body.as_bytes(),
    ))
}

/// The clone of `one_not_downloaded` whose Git LFS asks a server of the tests answering so, in order.
fn with_server(answers: &[(&str, &str)]) -> (Repository, PathBuf, TestServer) {
    let (repository, clone) = one_not_downloaded();
    let server = TestServer::plain(answers.iter().map(|(status, body)| lfs_answer(status, body)).collect());
    let url = format!("http://127.0.0.1:{}/lfs", server.port());
    let _ = git_in(&repository, &clone, &["config", "lfs.url", &url]);
    (repository, clone, server)
}

/// The requests to the API of the locks the server received.
fn lock_requests(server: &TestServer) -> Vec<String> {
    server
        .received()
        .iter()
        .map(http::test_server::Received::request_line)
        .filter(|line| line.contains("/locks"))
        .collect()
}

/// The page of a folder, opened in a window as the tests of the page open it.
fn opened(folder: &Path) -> (State, super::support::Window) {
    page(folder, ThemeName::Dark)
}

#[test]
fn lfs_read_locks__opening_the_large_files_page_runs_it_once_and_a_read_of_the_repository_never_does() {
    let (_repository, clone, server) = with_server(&[("200 OK", NO_LOCK), ("200 OK", NO_LOCK), ("200 OK", NO_LOCK)]);
    let (mut state, _window) = opened(&clone);
    assert_eq!(lock_requests(&server), ["POST /lfs/locks/verify HTTP/1.1"]);
    let _ = run(&mut state, Message::Intent(Intent::Refresh));
    let _ = run(&mut state, Message::Intent(Intent::Refresh));
    assert_eq!(lock_requests(&server).len(), 1, "{:?}", lock_requests(&server));
}

#[test]
fn lfs_page__a_file_locked_by_someone_else_shows_their_name_and_its_menu_has_unlock_with_the_warning() {
    let (_repository, clone, _server) = with_server(&[("200 OK", THEIRS), ("200 OK", THEIRS)]);
    let (mut state, mut window) = opened(&clone);
    let tree = drawn(&mut window, &state);
    assert!(says(&tree, "reviewer"));
    let items = menu_of(&mut window, &state, "art/logo.psd");
    assert!(items.contains(&String::from(words::UNLOCK)), "{items:?}");
    assert!(!items.contains(&String::from(words::LOCK)), "{items:?}");
    let driver = OperationsDriver::new();
    for intent in window.click(&state, Role::MenuItem, words::UNLOCK) {
        driver.intent(&mut state, intent);
    }
    let Message::FeatureConfirms(plan) = driver.next(&mut state) else {
        panic!("someone else's lock is shown first");
    };
    assert_eq!(views::feature::plan_lines(&plan), ["git lfs unlock --force --id=7"]);
    let warning = "The lock is reviewer's, and they are not told: they may be changing the file.";
    assert!(
        matches!(&plan.danger, Danger::Confirm(said) if said.to_string() == warning),
        "{:?}",
        plan.danger
    );
    driver.intent(&mut state, Intent::Feature(FeatureIntent::Confirm(false)));
}

#[test]
fn lfs_lock__lock_on_art_logo_psd_runs_git_lfs_lock_json_and_the_page_then_shows_the_lock_as_this_users() {
    let (_repository, clone, server) = with_server(&[
        ("200 OK", NO_LOCK),
        ("201 Created", LOCKED),
        ("200 OK", OURS),
        ("200 OK", OURS),
    ]);
    let (mut state, mut window) = opened(&clone);
    let items = menu_of(&mut window, &state, "art/logo.psd");
    assert!(items.contains(&String::from(words::LOCK)), "{items:?}");
    let driver = OperationsDriver::new();
    for intent in window.click(&state, Role::MenuItem, words::LOCK) {
        driver.intent(&mut state, intent);
    }
    let log = loop {
        match driver.next(&mut state) {
            Message::FeatureConfirms(_) => driver.intent(&mut state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureEnded { ended, log } => {
                assert!(matches!(ended, app::store::feature::Ended::Ran { .. }), "{ended:?}");
                break log;
            }
            _ => {}
        }
    };
    let lines: Vec<String> = log.into_iter().map(|entry| entry.line).collect();
    assert_eq!(lines, ["git lfs lock --json art/logo.psd"]);
    let tree = drawn(&mut window, &state);
    assert!(says(&tree, words::YOU), "{:?}", lock_requests(&server));
    let items = menu_of(&mut window, &state, "art/logo.psd");
    assert!(items.contains(&String::from(words::UNLOCK)), "{items:?}");
}
