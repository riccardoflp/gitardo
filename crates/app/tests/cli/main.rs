//! The features run by `gitardo-cli` on throwaway repositories: the engine, the features and git together, as the
//! window will run them.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
#![allow(clippy::panic, reason = "a helper that cannot go on fails the test")]

mod branch;
mod branches;
mod changes;
mod checkout;
mod commit;
mod commits;
mod compare;
mod conflicts;
mod merge;
mod message;
mod patch;
mod picks;
mod push;
mod rebase;
mod remote;
mod rewrite;
mod stash;
mod state;
mod submodule;
mod sync;
mod sync_all;
mod tag;
mod undo;
mod worktree_setup;

use testkit::Repository;

/// How `gitardo-cli` ended, and what it printed.
struct Ended {
    /// Its exit code: 0 done, 1 refused or failed, 2 an answer or a confirmation is missing.
    code: Option<i32>,
    /// What it printed on its standard output.
    out: String,
    /// What it printed on its standard error.
    errors: String,
}

/// Runs `gitardo-cli` in the repository's folder, in the environment of its git commands.
fn cli(repository: &Repository, arguments: &[&str]) -> Ended {
    let output = match repository
        .command(env!("CARGO_BIN_EXE_gitardo-cli"))
        .args(arguments)
        .output()
    {
        Ok(output) => output,
        Err(error) => panic!("could not run gitardo-cli: {error}"),
    };
    Ended {
        code: output.status.code(),
        out: String::from_utf8_lossy(&output.stdout).into_owned(),
        errors: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// Runs `gitardo-cli` and fails the test, with what it printed, unless it did what it was asked.
fn cli_done(repository: &Repository, arguments: &[&str]) -> Ended {
    let ended = cli(repository, arguments);
    assert_eq!(
        ended.code,
        Some(0),
        "gitardo-cli {} failed:\n{}\n{}",
        arguments.join(" "),
        ended.out,
        ended.errors
    );
    ended
}

/// The full id of a revision.
fn revision(repository: &Repository, name: &str) -> String {
    repository.git(&["rev-parse", name]).trim_end().to_owned()
}

/// The branch checked out, or `HEAD` when detached.
fn checked_out(repository: &Repository) -> String {
    repository
        .git(&["rev-parse", "--abbrev-ref", "HEAD"])
        .trim_end()
        .to_owned()
}

/// Writes a file of the working tree.
fn write(repository: &Repository, name: &str, content: &str) {
    let path = repository.path().join(name);
    if let Err(error) = std::fs::write(&path, content) {
        panic!("could not write {}: {error}", path.display());
    }
}
