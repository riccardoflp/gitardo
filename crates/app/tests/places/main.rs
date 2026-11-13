//! The places of the window drawn from the store, with repositories made for each test: what they show, the pictures of
//! them in each theme, what a click on them asks, and the reads of the workers that fill them.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
#![allow(clippy::unwrap_used, clippy::panic, reason = "a test that cannot go on fails")]

mod activity;
mod bisect;
mod branches;
mod changes;
mod commit_page;
mod compare;
mod compare_actions;
mod conflicts;
mod diff;
mod diff_links;
mod diff_staging;
mod diff_views;
mod filter;
mod folds;
mod go_to;
mod histories;
mod history;
mod inspector;
mod inspector_files;
mod jobs;
mod large_diffs;
mod merge;
mod merge_actions;
mod operations;
mod panes;
mod patches;
mod pointer;
mod rail;
mod rebase;
mod review;
mod review_cards;
mod search;
mod show_in_graph;
mod speed;
mod stashes;
mod support;
mod sync_stash;
mod timeline;
mod views_drawn;
mod watch;

use std::path::PathBuf;

use app::persisted::{Persisted, RecentWorkspace};
use app::settings::Settings;
use app::store::{Message, State};
use testkit::{Remote, Repository};

/// A window that started before, with no workspace and two recent ones.
fn window() -> State {
    State::new(
        Persisted {
            has_started: true,
            recent: vec![
                RecentWorkspace {
                    folders: vec![PathBuf::from("/work/app")],
                    repository: None,
                },
                RecentWorkspace {
                    folders: vec![PathBuf::from("/work/site"), PathBuf::from("/work/docs")],
                    repository: None,
                },
            ],
            ..Persisted::default()
        },
        Settings::default(),
        None,
    )
}

/// A repository with some history: `main` pushed to `origin` and one commit ahead of it, a branch `feature`, a tag
/// `v1.0` and a stash.
fn repository_with_history() -> (Repository, Remote) {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let _ = repository.commit("Read the settings at startup", &[("settings.txt", "theme\n")]);
    repository.tag("v1.0");
    let remote = Remote::new(&repository);
    repository.branch("feature");
    let _ = repository.commit("Show the branch rail", &[("rail.txt", "branches\n")]);
    std::fs::write(repository.path().join("rail.txt"), "branches and tags\n").unwrap();
    let _ = repository.git(&["stash", "push", "--quiet", "--message", "half the tags"]);
    (repository, remote)
}

/// A window showing a workspace of one folder, its jobs run.
fn showing(folder: PathBuf) -> State {
    let mut state = window();
    let _effects = support::run(&mut state, Message::Started { folders: vec![folder] });
    state
}
