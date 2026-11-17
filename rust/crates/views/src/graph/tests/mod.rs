//! The timeline drawn from rows made by hand: the keys, the pages asked for, the label under the pointer, the marks,
//! the sticky names, the menus of the draft, a stash and a merge, and the panes; here, the rows and the repository
//! they are made of.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::collections::HashSet;
use std::fmt::Write as _;

use model::commit::Commit;
use model::contract::OpenIn;
use model::graph::Row;
use model::state::{Branch, Head, RepositoryState};
use model::{CommitId, Timestamp};
use ui::{Key, KeyEvent, Modifiers, NamedKey};

use super::intent::{GraphIntent, Selected, Switches};
use super::layout::{LayoutInput, TimelineLayout};
use super::state::{DraftState, Folds, HistoryState, Reading};
use crate::branch::ReferenceMenus;

mod keys_and_pages;
mod menus_and_panes;
mod one_lane;
mod selection;

/// The hash of a commit named in a test: its name in hexadecimal, padded.
pub(super) fn id(name: &str) -> CommitId {
    let hex = name.bytes().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    });
    CommitId::parse(&format!("{hex:0<40}")).unwrap_or_else(|| panic!("{name} makes no hash"))
}

/// A commit row, `name: parents`.
pub(super) fn commit(name: &str, parents: &[&str]) -> Row {
    Row::Commit(Commit {
        id: id(name),
        parents: parents.iter().map(|parent| id(parent)).collect(),
        author_name: String::from("Author"),
        author_email: String::new(),
        author_date: Timestamp(0),
        subject: name.to_owned(),
    })
}

/// Rows, each `name: parents`.
pub(super) fn rows(spec: &[&str]) -> Vec<Row> {
    spec.iter()
        .map(|line| {
            let (name, parents) = line.split_once(':').unwrap_or((line, ""));
            commit(name.trim(), &parents.split_whitespace().collect::<Vec<_>>())
        })
        .collect()
}

/// A repository checked out at a commit, with branches at some commits.
pub(super) fn repository(head: &str, branches: &[(&str, &str)]) -> RepositoryState {
    RepositoryState {
        head: Head {
            commit: Some(id(head)),
            branch: Some(String::from("refs/heads/main")),
        },
        branches: branches
            .iter()
            .map(|(name, at)| Branch {
                name: (*name).to_owned(),
                commit: id(at),
                upstream: None,
                is_checked_out: *name == "main",
                date: Timestamp(0),
            })
            .collect(),
        ..RepositoryState::default()
    }
}

/// Everything a timeline is drawn from, owned.
pub(super) struct Fixture {
    pub(super) rows: Vec<Row>,
    pub(super) layout: TimelineLayout,
    folded: HashSet<CommitId>,
    selected: Option<Selected>,
    pub(crate) several: Vec<CommitId>,
    draft: Option<DraftState>,
    repository: RepositoryState,
}

impl Fixture {
    pub(super) fn new(rows: Vec<Row>, repository: &RepositoryState, selected: Option<Selected>) -> Fixture {
        let folded = HashSet::new();
        let layout = TimelineLayout::new(&LayoutInput {
            rows: &rows,
            repository,
            folded: &folded,
            combine_upstream: true,
            separators_at: None,
            one_lane: false,
        });
        let draft = rows
            .iter()
            .any(|row| matches!(row, Row::Uncommitted { .. }))
            .then(DraftState::default);
        Fixture {
            rows,
            layout,
            folded,
            selected,
            several: Vec::new(),
            draft,
            repository: repository.clone(),
        }
    }

    pub(super) fn history(&self) -> HistoryState<'_> {
        HistoryState {
            rows: &self.rows,
            layout: &self.layout,
            folds: Folds {
                folded: &self.folded,
                is_all_folded: false,
            },
            has_no_commits: false,
            has_more: false,
            reading: Reading::Nothing,
            asked_at: None,
            selected: self.selected.as_ref(),
            several: &self.several,
            reveal: None,
            trace: None,
            hover: None,
            hover_left_at: None,
            label_list: None,
            label_list_closed_at: None,
            rail_hover: None,
            dropped: None,
            switches: Switches::DEFAULT,
            graph_width: None,
            now: 0,
            found: &[],
            dims_others: false,
            draft: self.draft,
            references: Some(ReferenceMenus {
                repository: &self.repository,
                open_in: OpenIn::Checkout,
                window: crate::worktree::WorktreeWindow::default(),
                github: crate::github::GitHubShown::default(),
            }),
            commit_box: crate::changes::CommitBox::default(),
        }
    }
}

/// A key pressed with some modifiers.
pub(super) fn key(named: NamedKey, modifiers: Modifiers) -> KeyEvent {
    KeyEvent::press(Key::Named(named), modifiers)
}

/// What selecting a commit by its name asks.
pub(super) fn select(name: &str) -> GraphIntent {
    GraphIntent::Select(Selected::Commit(id(name)))
}

/// A timeline where `feature` left `main` at `A`, with `main` going on under its tip.
pub(super) fn feature_and_main() -> Fixture {
    Fixture::new(
        rows(&["F2: F1", "M3: M2", "F1: A", "M2: M1", "M1: A", "A:"]),
        &repository("M3", &[("main", "M3"), ("feature", "F2")]),
        None,
    )
}

#[test]
fn refs_tree__each_local_branch_is_in_the_color_of_its_lane() {
    let rows = rows(&["F2: F1", "M3: M2", "F1: A", "M2: M1", "M1: A", "A:"]);
    let repository = repository("M3", &[("main", "M3"), ("feature", "F2"), ("old", "Z")]);
    let fixture = Fixture::new(rows, &repository, None);
    let layout = &fixture.layout;
    let color_of_commit = |name: &str| {
        let shown = layout.shown_of.get(&id(name)).copied();
        shown.and_then(|shown| layout.cells.get(shown)).map(|cell| cell.color)
    };
    assert_eq!(layout.branch_colors.get("main").copied(), color_of_commit("M3"));
    assert_eq!(layout.branch_colors.get("feature").copied(), color_of_commit("F2"));
    assert_ne!(layout.branch_colors.get("main"), layout.branch_colors.get("feature"));
    assert_eq!(
        layout.branch_colors.get("old"),
        None,
        "a branch whose commit is not loaded has no color"
    );
}
