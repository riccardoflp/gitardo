//! The parsers of `repo.read-state` on what git printed in awkward repositories, kept in `tests/golden/repo-state`:
//! `<case>.out` is what one command printed, `<case>.expected` what its parser makes of it.
//!
//! With `GITARDO_GOLDEN_RECORD=1`, each test makes its repository, runs the command with the arguments of the read and
//! records both files again; otherwise the parsers read the files of the repository, without git.

use std::ffi::OsString;
use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;

use git::GitError;
use git::read::repo::{
    identity_arguments, location_arguments, parse_identity, parse_location, parse_references, parse_reflog_subjects,
    parse_remotes, parse_stashes, parse_status, parse_worktrees, references_arguments, reflog_subjects_arguments,
    remotes_arguments, stashes_arguments, status_arguments, worktrees_arguments,
};
use testkit::{Remote, Repository, assert_golden};

use crate::{run_stopping, write};

/// The file of a case, with its extension.
fn golden_file(case: &str, extension: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/repo-state")
        .join(format!("{case}.{extension}"))
}

/// Whether the golden files are being recorded again.
fn is_recording() -> bool {
    std::env::var_os("GITARDO_GOLDEN_RECORD").is_some_and(|value| value == "1")
}

/// When recording, makes the repository of a case, runs a read's arguments there with the configuration every read
/// gets, and keeps what git printed in `<case>.out`, with the folder of the test written `<folder>` so that the file is
/// the same on every machine.
fn record(case: &str, repository: impl FnOnce() -> Repository, arguments: &[OsString]) {
    if !is_recording() {
        return;
    }
    let repository = repository();
    let mut words = vec!["-c", "core.quotepath=false"];
    words.extend(arguments.iter().map(|argument| argument.to_str().unwrap()));
    let printed = repository.git(&words);
    let folder = repository.path().parent().unwrap().to_string_lossy().replace('\\', "/");
    assert_golden(golden_file(case, "out"), printed.replace(&folder, "<folder>"));
}

/// Parses `<case>.out` and compares what the parser made of it with `<case>.expected`.
fn check<Parsed: Debug>(case: &str, parse: impl Fn(&[u8]) -> Result<Parsed, GitError>) {
    let output = fs::read(golden_file(case, "out")).unwrap();
    let parsed = parse(&output).unwrap();
    assert_golden(golden_file(case, "expected"), format!("{parsed:#?}\n"));
}

/// File names with spaces, accents, brackets and a leading dash, and one that is also a pathspec pattern.
const AWKWARD_FILES: [(&str, &str); 6] = [
    ("with space.txt", "1\n"),
    ("accentué.txt", "1\n"),
    ("pages/[id].tsx", "1\n"),
    ("-leading-dash.txt", "1\n"),
    ("renamed from.txt", "one\ntwo\nthree\nfour\n"),
    ("deleted.txt", "1\n"),
];

/// Branches with awkward names, a branch and a tag of the same name, a branch and a remote branch of the same name, an
/// annotated tag, `origin/HEAD`, a branch ahead and behind its upstream and one whose upstream is gone.
fn references_repository() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the files", &AWKWARD_FILES);
    let remote = Remote::new(&repository);
    let _ = repository.git(&["remote", "set-head", "origin", "main"]);
    for name in ["release", "caffè", "fix/with]bracket", "origin/main", "feature"] {
        repository.branch(name);
    }
    let _ = repository.git(&["update-ref", "refs/heads/-leading-dash", "HEAD"]);
    repository.tag("release");
    let _ = repository.git(&["tag", "--annotate", "v1.0.0", "--message", "The first release"]);
    let _ = repository.git(&["push", "--quiet", "--set-upstream", "origin", "feature"]);
    let _ = remote.git(&["branch", "--quiet", "-D", "feature"]);
    let _ = repository.git(&["fetch", "--quiet", "--prune"]);
    let _ = repository.commit("Change on the remote", &[("remote.txt", "1\n")]);
    let _ = repository.git(&["push", "--quiet", "origin", "main"]);
    let _ = repository.git(&["reset", "--quiet", "--hard", "HEAD~1"]);
    let _ = repository.commit("First change here", &[("local.txt", "1\n")]);
    let _ = repository.commit("Second change here", &[("local.txt", "2\n")]);
    repository
}

/// A detached `HEAD`, with branches and a tag.
fn detached_repository() -> Repository {
    let mut repository = Repository::new();
    let first = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    repository.tag("v0.1.0");
    repository.branch("feature");
    let _ = repository.commit("Fix the parser", &[("parser.rs", "2\n")]);
    repository.checkout(&first);
    repository
}

/// Awkward file names staged, changed, renamed, deleted and untracked.
fn status_repository() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the files", &AWKWARD_FILES);
    write(&repository, "with space.txt", "2\n");
    write(&repository, "accentué.txt", "2\n");
    write(&repository, "pages/[id].tsx", "2\n");
    let _ = repository.git(&["add", "--", ":(literal)accentué.txt", ":(literal)pages/[id].tsx"]);
    write(&repository, "pages/[id].tsx", "3\n");
    let _ = repository.git(&["mv", "--", "renamed from.txt", "renamed to.txt"]);
    let _ = repository.git(&["rm", "--quiet", "--", "deleted.txt"]);
    write(&repository, "new folder/[draft].md", "draft\n");
    write(&repository, "-new.txt", "new\n");
    repository
}

/// A repository with no commits and untracked files.
fn empty_repository() -> Repository {
    let repository = Repository::new();
    write(&repository, "notes.txt", "a\n");
    write(&repository, "src/parser.rs", "fn parse() {}\n");
    repository
}

/// A merge stopped on conflicts of four kinds: changed on both sides, added on both sides, deleted by them and
/// deleted by us.
fn conflicts_repository() -> Repository {
    let mut repository = Repository::new();
    let base = [
        ("both.txt", "base\n"),
        ("deleted by them.txt", "base\n"),
        ("deleted by us.txt", "base\n"),
    ];
    let _ = repository.commit("Add the files", &base);
    repository.branch("theirs");
    let _ = repository.git(&["rm", "--quiet", "--", "deleted by us.txt"]);
    let ours = [
        ("both.txt", "ours\n"),
        ("deleted by them.txt", "ours\n"),
        ("added.txt", "ours\n"),
    ];
    let _ = repository.commit("Change on main", &ours);
    repository.checkout("theirs");
    let _ = repository.git(&["rm", "--quiet", "--", "deleted by them.txt"]);
    let theirs = [
        ("both.txt", "theirs\n"),
        ("deleted by us.txt", "theirs\n"),
        ("added.txt", "theirs\n"),
    ];
    let _ = repository.commit("Change on theirs", &theirs);
    repository.checkout("main");
    run_stopping(&repository, &["merge", "--no-edit", "theirs"]);
    repository
}

/// The main worktree, one on a branch, one detached and locked, and one whose folder is gone.
fn worktrees_repository() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    repository.branch("feature");
    repository.branch("gone");
    let _ = repository.git(&["worktree", "add", "--quiet", "../feature with space", "feature"]);
    let _ = repository.git(&["worktree", "add", "--quiet", "--detach", "../détaché"]);
    let _ = repository.git(&["worktree", "lock", "--reason", "on a drive [unplugged]", "../détaché"]);
    let _ = repository.git(&["worktree", "add", "--quiet", "../gone", "gone"]);
    fs::remove_dir_all(repository.path().parent().unwrap().join("gone")).unwrap();
    repository
}

/// Two stashes, one with the untracked files, with awkward messages.
fn stashes_repository() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    write(&repository, "parser.rs", "2\n");
    let _ = repository.git(&["stash", "push", "--quiet", "--message", "before the merge: [draft] é"]);
    write(&repository, "-new.txt", "new\n");
    let _ = repository.git(&["stash", "push", "--quiet", "--include-untracked"]);
    repository
}

/// Remotes, one with a `/` in its name.
fn remotes_repository() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let remote = Remote::new(&repository);
    let path = remote.path().to_str().unwrap().to_owned();
    let _ = repository.git(&["remote", "add", "my/fork", &path]);
    let _ = repository.git(&["remote", "add", "upstream", &path]);
    repository
}

/// An identity with an accent and brackets.
fn identity_repository() -> Repository {
    let repository = Repository::new();
    let _ = repository.git(&["config", "user.name", "Zoë [Example]"]);
    let _ = repository.git(&["config", "user.email", "zoe@example.com"]);
    repository
}

/// Commits, a checkout, a merge, a reset and a detached `HEAD`, each in the reflog.
fn reflog_repository() -> Repository {
    let mut repository = Repository::new();
    let first = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    repository.branch("feature/é");
    repository.checkout("feature/é");
    let _ = repository.commit("Change [the] parser", &[("parser.rs", "2\n")]);
    repository.checkout("main");
    repository.merge("feature/é");
    let _ = repository.git(&["reset", "--quiet", "--hard", "HEAD~1"]);
    repository.checkout(&first);
    repository
}

#[test]
fn repo_read_state__golden_references_of_awkward_names_upstreams_and_tags() {
    record("references-awkward", references_repository, &references_arguments());
    check("references-awkward", parse_references);
}

#[test]
fn repo_read_state__golden_references_when_detached() {
    record("references-detached", detached_repository, &references_arguments());
    check("references-detached", parse_references);
}

#[test]
fn repo_read_state__golden_references_of_an_empty_repository() {
    record("references-empty", Repository::new, &references_arguments());
    check("references-empty", parse_references);
}

#[test]
fn repo_read_state__golden_status_of_awkward_names() {
    record("status-awkward", status_repository, &status_arguments());
    check("status-awkward", parse_status);
}

#[test]
fn repo_read_state__golden_status_of_an_empty_repository() {
    record("status-empty", empty_repository, &status_arguments());
    check("status-empty", parse_status);
}

#[test]
fn repo_read_state__golden_status_of_a_merge_stopped_on_conflicts() {
    record("status-conflicts", conflicts_repository, &status_arguments());
    check("status-conflicts", parse_status);
}

#[test]
fn repo_read_state__golden_worktrees() {
    record("worktrees", worktrees_repository, &worktrees_arguments());
    check("worktrees", parse_worktrees);
}

#[test]
fn repo_read_state__golden_worktrees_of_an_empty_repository() {
    record("worktrees-empty", Repository::new, &worktrees_arguments());
    check("worktrees-empty", parse_worktrees);
}

#[test]
fn repo_read_state__golden_stashes() {
    record("stashes", stashes_repository, &stashes_arguments());
    check("stashes", parse_stashes);
}

#[test]
fn repo_read_state__golden_remotes() {
    record("remotes", remotes_repository, &remotes_arguments());
    check("remotes", parse_remotes);
}

#[test]
fn repo_read_state__golden_identity() {
    record("identity", identity_repository, &identity_arguments());
    check("identity", parse_identity);
}

#[test]
fn repo_read_state__golden_reflog_subjects() {
    record("reflog-subjects", reflog_repository, &reflog_subjects_arguments());
    check("reflog-subjects", parse_reflog_subjects);
}

#[test]
fn repo_read_state__golden_location() {
    record("location", Repository::new, &location_arguments());
    check("location", parse_location);
}
