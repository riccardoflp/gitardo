use model::contract::{Answers, Outcome, Target};
use model::lfs::{LfsDownload, LfsPath, LfsStatus};
use model::state::RepositoryState;

use super::download;
use crate::testing::{commit, context, lines, plan, state};

/// A repository with Git LFS 3.7.1 and the remote `origin`.
fn with_lfs() -> RepositoryState {
    RepositoryState {
        lfs: LfsStatus {
            version: Some(String::from("3.7.1")),
            is_set_up: true,
            patterns: Vec::new(),
        },
        remotes: vec![String::from("origin")],
        ..state()
    }
}

/// What **Download** is asked to download.
fn files(paths: &[(&str, bool)], commit: Option<char>) -> Target {
    Target::LfsFiles(Box::new(LfsDownload {
        paths: paths
            .iter()
            .map(|(path, is_folder)| LfsPath {
                path: (*path).to_owned(),
                is_folder: *is_folder,
            })
            .collect(),
        commit: commit.map(crate::testing::commit),
    }))
}

#[test]
fn lfs_download__a_file_of_the_commit_checked_out_is_pulled_with_its_path_included() {
    let ran = plan(download(
        &context(with_lfs(), files(&[("art/logo.psd", false)], None)),
        &Answers::default(),
    ));
    assert_eq!(lines(&ran), ["git lfs pull --include=art/logo.psd"]);
    assert_eq!(ran.title.as_str(), "Download art/logo.psd with Git LFS");
    // The commit checked out named by its id is pulled all the same.
    let named = plan(download(
        &context(with_lfs(), files(&[("art/logo.psd", false)], Some('a'))),
        &Answers::default(),
    ));
    assert_eq!(lines(&named), ["git lfs pull --include=art/logo.psd"]);
}

#[test]
fn lfs_download__the_folder_art_is_pulled_as_art_and_two_stars() {
    let ran = plan(download(
        &context(with_lfs(), files(&[("art", true)], None)),
        &Answers::default(),
    ));
    assert_eq!(lines(&ran), ["git lfs pull --include=art/**"]);
}

#[test]
fn lfs_download__download_all_pulls_every_file() {
    let ran = plan(download(&context(with_lfs(), files(&[], None)), &Answers::default()));
    assert_eq!(lines(&ran), ["git lfs pull"]);
    assert_eq!(ran.title.as_str(), "Download every file stored with Git LFS");
}

#[test]
fn lfs_download__a_file_of_a_commit_not_checked_out_is_fetched_from_origin_for_that_commit() {
    let ran = plan(download(
        &context(with_lfs(), files(&[("art/logo.psd", false)], Some('b'))),
        &Answers::default(),
    ));
    assert_eq!(
        lines(&ran),
        [format!("git lfs fetch --include=art/logo.psd origin {}", commit('b'))]
    );
    // The upstream of the branch checked out names the remote before the default one.
    let mut upstream = with_lfs();
    upstream.remotes.push(String::from("fork"));
    upstream.branches[0].upstream = Some(crate::testing::tracking("fork", "main"));
    let ran = plan(download(
        &context(upstream, files(&[("art/logo.psd", false)], Some('b'))),
        &Answers::default(),
    ));
    assert!(lines(&ran)[0].contains(" fork "), "{:?}", lines(&ran));
}

#[test]
fn lfs_download__without_git_lfs_it_refuses_with_the_message_and_runs_nothing() {
    let without = RepositoryState {
        lfs: LfsStatus::default(),
        ..with_lfs()
    };
    let Outcome::Refuse(refusal) = download(&context(without, files(&[], None)), &Answers::default()) else {
        panic!("it refuses");
    };
    assert_eq!(
        refusal.message.as_str(),
        "Git LFS is not installed: install it from git-lfs.com, then try again."
    );
    let Outcome::Refuse(refusal) = download(
        &context(with_lfs(), files(&[("logo, final.psd", false)], None)),
        &Answers::default(),
    ) else {
        panic!("a path with a comma is refused");
    };
    assert!(refusal.message.as_str().starts_with("logo, final.psd cannot be named"));
}
