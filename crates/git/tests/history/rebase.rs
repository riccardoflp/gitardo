//! `rebase.read-pushed` and `rebase.read-stop` against throwaway repositories.

use std::fs;

use git::read::rebase::{PushedRequest, StopRequest, read_pushed, read_stop};
use model::CommitId;
use model::rebase::{RebaseStop, StopKind, pushed_note};
use testkit::{Remote, Repository};

use crate::{git, run, write};

/// The remote branches that have a commit.
fn pushed(repository: &Repository, commit: &str) -> Vec<String> {
    let request = PushedRequest {
        commit: CommitId::parse(commit).unwrap(),
    };
    read_pushed(&git(repository), &request).unwrap()
}

/// A commit pushed to `main` of a remote, and to other branches of it.
fn pushed_to(branches: &[&str]) -> (Repository, String) {
    let mut repository = Repository::new();
    let commit = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let _remote = Remote::new(&repository);
    let _ = repository.git(&["remote", "set-head", "origin", "main"]);
    for branch in branches {
        let _ = repository.git(&["push", "--quiet", "origin", &format!("HEAD:refs/heads/{branch}")]);
    }
    let _ = repository.git(&["fetch", "--quiet"]);
    (repository, commit)
}

#[test]
fn rebase_read_pushed__gives_every_remote_branch_with_the_commit() {
    let (repository, commit) = pushed_to(&["feature"]);
    let branches = pushed(&repository, &commit);
    assert_eq!(branches, ["origin/feature", "origin/main"]);
    let note = pushed_note(&CommitId::parse(&commit).unwrap(), &branches)
        .unwrap()
        .to_string();
    assert!(
        note.contains("on origin/feature, origin/main: the branch will have to be force pushed"),
        "{note}"
    );
}

#[test]
fn rebase_read_pushed__names_three_branches_and_counts_the_others() {
    let (repository, commit) = pushed_to(&["a", "b", "c", "d"]);
    let branches = pushed(&repository, &commit);
    assert_eq!(branches.len(), 5);
    let note = pushed_note(&CommitId::parse(&commit).unwrap(), &branches)
        .unwrap()
        .to_string();
    assert!(note.contains("on origin/a, origin/b, origin/c and 2 more: "), "{note}");
}

#[test]
fn rebase_read_pushed__gives_nothing_for_a_commit_on_no_remote_branch() {
    let (mut repository, _) = pushed_to(&[]);
    let local = repository.commit("Fix the parser", &[("parser.rs", "2\n")]);
    let branches = pushed(&repository, &local);
    assert_eq!(branches, Vec::<String>::new());
    assert_eq!(pushed_note(&CommitId::parse(&local).unwrap(), &branches), None);
}

/// Where the rebase of a repository stopped.
fn stop(repository: &Repository) -> Option<RebaseStop> {
    let request = StopRequest {
        git_folder: repository.path().join(".git"),
    };
    read_stop(&git(repository), &request).unwrap()
}

/// Three commits, then an interactive rebase of the last two, stopped to edit the first of them, `B`, whose hash is
/// returned. `C` changes `shared.txt`.
fn stopped_on_edit() -> (Repository, String) {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the files", &[("shared.txt", "base\n")]);
    let edited = repository.commit("Add the login page", &[("login.rs", "1\n")]);
    let _ = repository.commit("Change the shared file", &[("shared.txt", "theirs\n")]);
    let editor = "sed -i -e 1s/^pick/edit/";
    let _ = repository.git(&[
        "-c",
        &format!("sequence.editor={editor}"),
        "rebase",
        "--quiet",
        "-i",
        "HEAD~2",
    ]);
    (repository, edited)
}

#[test]
fn rebase_read_stop__gives_an_edit_of_the_commit_in_amend_with_its_subject() {
    let (repository, edited) = stopped_on_edit();
    let expected = RebaseStop {
        kind: StopKind::Edit,
        commit: CommitId::parse(&edited).unwrap(),
        subject: String::from("Add the login page"),
    };
    assert_eq!(stop(&repository), Some(expected));
}

#[test]
fn rebase_read_stop__gives_nothing_when_a_later_pick_stops_on_a_conflict() {
    let (repository, edited) = stopped_on_edit();
    write(&repository, "shared.txt", "ours\n");
    let _ = repository.git(&["commit", "--quiet", "--all", "--amend", "--no-edit"]);
    let continued = run(&repository, &["-c", "core.editor=true", "rebase", "--continue"], b"");
    assert!(
        !continued.status.success(),
        "the pick of the last commit did not stop on a conflict"
    );
    // Some versions of git leave `amend` after the edit, as the card says, and some remove it: it is put back, so that
    // the stop is told from `done` either way.
    fs::write(repository.path().join(".git/rebase-merge/amend"), format!("{edited}\n")).unwrap();
    assert_eq!(stop(&repository), None);
}

#[test]
fn rebase_read_stop__gives_a_split_when_gitardo_split_holds_the_commit_of_the_last_edit() {
    let (repository, edited) = stopped_on_edit();
    fs::write(
        repository.path().join(".git/rebase-merge/gitardo-split"),
        format!("{edited}\n"),
    )
    .unwrap();
    let found = stop(&repository).unwrap();
    assert_eq!((found.kind, found.commit.as_str()), (StopKind::Split, edited.as_str()));
}

#[test]
fn rebase_read_stop__gives_nothing_with_no_rebase_under_way() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    assert_eq!(stop(&repository), None);
}
