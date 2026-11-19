//! The token of a GitLab, read with a credential helper of the test's own for `gitlab.com` alone, in a throwaway
//! repository, away from the configuration and the credential helper of the person running the tests.

use std::fs;
use std::path::Path;

use git::Git;
use git::read::hosting::{HostingTokenRequest, parse_default_branch, read_token, token_question};
use model::github::{HelperAsked, Token};
use testkit::Repository;

use super::beside;

/// What the helper of the tests prints for `get`, and the file it writes each time it runs.
const HELPER: &str = "!f() { echo ran >> helper-ran; echo username=ada; echo password=glpat-from-helper; }; f";

/// A git that reads no configuration but the repository's.
fn isolated(repository: &Repository) -> Git {
    let global = beside(repository).join("empty.gitconfig");
    fs::write(&global, "").unwrap();
    Git::new(repository.path())
        .with_variable("GIT_CONFIG_NOSYSTEM", "1")
        .with_variable("GIT_CONFIG_GLOBAL", global.as_os_str())
}

/// A repository whose only credential helper is the test's, for `https://gitlab.com` alone.
fn with_gitlab_helper() -> Repository {
    let repository = Repository::new();
    let _ = repository.git(&["config", "--add", "credential.helper", ""]);
    let _ = repository.git(&["config", "credential.https://gitlab.com.helper", HELPER]);
    repository
}

fn helper_ran(path: &Path) -> bool {
    path.join("helper-ran").exists()
}

fn request(host: &str, kept: Option<&str>, helper: HelperAsked) -> HostingTokenRequest {
    HostingTokenRequest {
        host: host.to_owned(),
        kept: kept.map(|token| Token(token.to_owned())),
        helper,
    }
}

#[test]
fn hosting_token__a_token_kept_for_gitlab_com_is_given_without_running_git() {
    let repository = with_gitlab_helper();
    let kept = request("gitlab.com", Some("glpat-kept"), HelperAsked::Allowed);
    let token = read_token(&isolated(&repository), &kept).unwrap();
    assert_eq!(token, Some(Token(String::from("glpat-kept"))));
    assert!(!helper_ran(repository.path()));
}

#[test]
fn hosting_token__the_helper_that_knows_gitlab_com_gives_the_password_it_prints() {
    let repository = with_gitlab_helper();
    let allowed = request("gitlab.com", None, HelperAsked::Allowed);
    let token = read_token(&isolated(&repository), &allowed).unwrap();
    assert_eq!(token, Some(Token(String::from("glpat-from-helper"))));
    assert!(helper_ran(repository.path()));
    assert_eq!(token_question("gitlab.com"), b"protocol=https\nhost=gitlab.com\n\n");
}

#[test]
fn hosting_token__a_request_that_does_not_allow_it_never_runs_git_credential_fill() {
    let repository = with_gitlab_helper();
    let not_allowed = request("gitlab.com", None, HelperAsked::NotAllowed);
    assert_eq!(read_token(&isolated(&repository), &not_allowed).unwrap(), None);
    assert!(!helper_ran(repository.path()));
}

#[test]
fn hosting_token__the_token_of_one_host_name_is_never_given_for_another() {
    let repository = with_gitlab_helper();
    let other = request("git.example.com", None, HelperAsked::Allowed);
    assert_eq!(read_token(&isolated(&repository), &other).unwrap(), None);
    assert!(
        !helper_ran(repository.path()),
        "the helper of gitlab.com is not asked for another host"
    );
}

#[test]
fn hosting_detect_remote__the_default_branch_of_the_remote_is_read_without_the_remote() {
    assert_eq!(
        parse_default_branch(b"origin/develop\n", "origin"),
        Some(String::from("develop"))
    );
    assert_eq!(parse_default_branch(b"", "origin"), None);
    let mut repository = Repository::new();
    let _first = repository.commit("First", &[("a.txt", "a")]);
    let _ = repository.git(&["update-ref", "refs/remotes/origin/trunk", "HEAD"]);
    let git = isolated(&repository);
    assert_eq!(git::read::hosting::read_default_branch(&git, "origin").unwrap(), None);
    let _ = repository.git(&["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/trunk"]);
    assert_eq!(
        git::read::hosting::read_default_branch(&git, "origin").unwrap(),
        Some(String::from("trunk"))
    );
}
