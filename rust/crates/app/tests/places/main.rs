//! The places of the window drawn from the store, with repositories made for each test: what they show, the pictures of
//! them in each theme, what a click on them asks, and the reads of the workers that fill them.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
#![allow(clippy::unwrap_used, clippy::panic, reason = "a test that cannot go on fails")]

mod activity;
mod activity_burst;
mod activity_list;
mod activity_replay;
mod activity_tab;
mod ai;
mod am_stopped;
mod beside;
mod bisect;
mod blame;
mod blame_file;
mod branches;
mod brand;
mod changes;
mod changes_beside;
mod changes_menu;
mod changes_save;
mod changes_section;
mod close_tabs;
mod co_authors;
mod commit_options;
mod commit_page;
mod compact_rows;
mod compare;
mod compare_actions;
mod compare_chooser;
mod compare_file;
mod conflicts;
mod diff;
mod diff_edit;
mod diff_links;
mod diff_open;
mod diff_staging;
mod diff_views;
mod draft;
mod drag;
mod file_icons;
mod file_search;
mod file_search_open;
mod file_search_preview;
mod file_view;
mod file_view_tools;
mod files_changes;
mod files_page;
mod filter;
mod folds;
mod github;
mod go_to;
mod going_to;
mod group_tabs;
mod header_menus;
mod histories;
mod history;
mod inspector;
mod inspector_files;
mod islands;
mod jobs;
mod label_overflow;
mod lanes;
mod large_diffs;
mod load_more;
mod log_secrets;
mod markdown;
mod menu_icons;
mod merge;
mod merge_actions;
mod merge_at_line;
mod mermaid;
mod new_worktrees;
mod one_lane;
mod open_tabs;
mod operations;
mod orphan;
mod page_title;
mod page_trail;
mod pane;
mod panel;
mod panes;
mod patches;
mod pointer;
mod rail;
mod rail_filter;
mod range_diff;
mod rebase;
mod review;
mod review_cards;
mod review_choose;
mod review_columns;
mod review_entry;
mod review_wiring;
mod said;
mod same_history;
mod search;
mod selection;
mod series_continued;
mod settings_file;
mod settings_page;
mod settings_reveal;
mod settings_search;
mod settings_values;
mod show_in_graph;
mod side_column;
mod side_room;
mod signature;
mod speed;
mod stack_undo;
mod stashes;
mod submodules;
mod summaries;
mod support;
mod sync_stash;
mod tab_operations;
mod tabs;
mod tag_remote;
mod tasks;
mod tasks_depend;
mod tasks_exchange;
mod tasks_group_runs;
mod tasks_groups;
mod tasks_quick_run;
mod tasks_run;
mod tasks_stop;
mod tasks_tab;
mod tasks_view;
mod terminal;
mod timeline;
mod title_bar;
mod trust;
mod under_pointer;
mod views_drawn;
mod watch;
mod where_one_is;
mod worktrees;

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
                    ..RecentWorkspace::default()
                },
                RecentWorkspace {
                    folders: vec![PathBuf::from("/work/site"), PathBuf::from("/work/docs")],
                    repository: None,
                    ..RecentWorkspace::default()
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

/// Folds the **Changes** section of the side column, for a test of the branch rail under it in a low window, or of
/// the Changes page, which then lists the files alone.
fn fold_changes(state: &mut State) {
    let fold = views::graph::GraphIntent::FoldSection(views::graph::SideSection::Changes);
    let _effects = support::run(state, Message::Intent(views::Intent::Graph(fold)));
}

/// A window showing a workspace of one folder, its jobs run.
fn showing(folder: PathBuf) -> State {
    let mut state = window();
    let _effects = support::run(&mut state, Message::Started { folders: vec![folder] });
    state
}
