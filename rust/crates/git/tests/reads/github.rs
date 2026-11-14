//! The GitHub token, read with a credential helper of the test's own in a throwaway repository, away from the
//! configuration and the credential helper of the person running the tests.

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use git::Git;
use git::read::github::{HelperAsked, TokenRequest, parse_token, read_token};
use model::github::Token;
use testkit::Repository;

use super::beside;

/// What the helper of the tests prints for `get`, and the file it writes each time it runs.
const HELPER: &str = "!f() { echo ran >> helper-ran; echo username=ada; echo password=token-from-helper; }; f";

/// A git that reads no configuration but the repository's: not the system's, not the user's, so that the credential
/// helper of the person running the tests is never asked.
fn isolated(repository: &Repository) -> Git {
    let global = beside(repository).join("empty.gitconfig");
    fs::write(&global, "").unwrap();
    Git::new(repository.path())
        .with_variable("GIT_CONFIG_NOSYSTEM", "1")
        .with_variable("GIT_CONFIG_GLOBAL", global.as_os_str())
}

/// A repository whose credential helpers are those given: an empty one first, which forgets any helper named before.
fn with_helpers(helpers: &[&str]) -> Repository {
    let repository = Repository::new();
    let _ = repository.git(&["config", "--add", "credential.helper", ""]);
    for helper in helpers {
        let _ = repository.git(&["config", "--add", "credential.helper", helper]);
    }
    repository
}

fn helper_ran(path: &Path) -> bool {
    path.join("helper-ran").exists()
}

#[test]
fn github_token__a_token_kept_is_given_without_running_git() {
    let repository = with_helpers(&[HELPER]);
    let request = TokenRequest {
        kept: Some(Token(String::from("kept-token"))),
        helper: HelperAsked::Allowed,
    };
    let token = read_token(&isolated(&repository), &request).unwrap();
    assert_eq!(token, Some(Token(String::from("kept-token"))));
    assert!(!helper_ran(repository.path()));
}

#[test]
fn github_token__the_helper_that_knows_github_gives_the_password_it_prints() {
    let repository = with_helpers(&[HELPER]);
    let request = TokenRequest {
        kept: None,
        helper: HelperAsked::Allowed,
    };
    let token = read_token(&isolated(&repository), &request).unwrap();
    assert_eq!(token, Some(Token(String::from("token-from-helper"))));
    assert!(helper_ran(repository.path()));
}

#[test]
fn github_token__a_request_that_does_not_allow_it_never_runs_git_credential_fill() {
    let repository = with_helpers(&[HELPER]);
    let request = TokenRequest {
        kept: None,
        helper: HelperAsked::NotAllowed,
    };
    assert_eq!(read_token(&isolated(&repository), &request).unwrap(), None);
    assert!(!helper_ran(repository.path()));
}

#[test]
fn github_token__with_no_helper_and_nothing_kept_there_is_no_token_and_nothing_is_asked() {
    let repository = with_helpers(&[]);
    let request = TokenRequest {
        kept: None,
        helper: HelperAsked::Allowed,
    };
    let started = Instant::now();
    assert_eq!(read_token(&isolated(&repository), &request).unwrap(), None);
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "git asked nobody and waited for nothing"
    );
}

#[test]
fn github_token__what_git_prints_is_read_for_its_password_only() {
    let printed = b"protocol=https\nhost=github.com\nusername=ada\npassword=abc\r\n";
    assert_eq!(parse_token(printed), Some(Token(String::from("abc"))));
    assert_eq!(parse_token(b"protocol=https\nhost=github.com\n"), None);
    assert_eq!(parse_token(b"password=\n"), None);
    assert_eq!(format!("{:?}", Token(String::from("abc"))), "Token(<hidden>)");
}
