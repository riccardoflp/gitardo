//! The room the side column takes (`repo.side-column`): a repository with three worktrees and two local branches, its
//! rows and its branches, each measured as tall as it wants to be; and a row of **Repositories** on one line, its name
//! and what it is on cut when the section is narrow (`repo.tabs`).

use app::store::{Message, State, ViewData};
use model::repo::{SummaryHead, WorktreeSummary};
use platform::accesskit::{Role, TreeUpdate};
use render::{Rect, ThemeName};
use testkit::Repository;
use views::repo::{SideColumnState, SideSections, TabEntry, TabSummary, TabsState, side_column_view, tabs_view};
use views::text::repo as words;

use super::pointer::{bounds, rect};
use super::support::{self, NOW, Window};

/// How high the side column of the repository of these tests wanted to be before phase 14 of M16, its sections side by
/// side in a window tall enough for both: its rows, its branches, their headings and the line between them.
const BEFORE_PHASE_14: f32 = 572.0;

/// `api` on `main`, with its worktree `api-login` on `feature/login` and a worktree `api-review` detached at `main`:
/// three worktrees and two local branches. A window opened on it shows `api`.
fn three_worktrees() -> ((Repository, Repository), State) {
    let mut origin = Repository::new();
    let _ = origin.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let holder = Repository::new();
    let api = holder.path().join("clones").join("api");
    let _ = origin.git(&["clone", "--quiet", ".", api.to_str().unwrap()]);
    let in_api = |arguments: &[&str]| {
        let mut all = vec!["-C", api.to_str().unwrap()];
        all.extend_from_slice(arguments);
        let _ = origin.git(&all);
    };
    let login = holder.path().join("clones").join("api-login");
    let review = holder.path().join("clones").join("api-review");
    in_api(&[
        "worktree",
        "add",
        "--quiet",
        "-b",
        "feature/login",
        login.to_str().unwrap(),
    ]);
    in_api(&[
        "worktree",
        "add",
        "--quiet",
        "--detach",
        review.to_str().unwrap(),
        "main",
    ]);
    let mut state = super::window();
    let _ = support::run(&mut state, Message::Started { folders: vec![api] });
    ((origin, holder), state)
}

/// The foot of the lowest row or button drawn in a part of the side column, by the name of that part.
fn lowest_in(tree: &TreeUpdate, part: &str) -> f32 {
    let part = bounds(tree, Role::Navigation, part);
    tree.nodes
        .iter()
        .filter(|(_, node)| matches!(node.role(), Role::Button | Role::Label))
        .filter_map(|(_, node)| node.bounds().map(rect))
        .filter(|found| found.y >= part.y && found.bottom() <= part.bottom() + 0.5 && found.height <= 64.0)
        .map(Rect::bottom)
        .fold(0.0, f32::max)
}

/// The side column of a state drawn alone, with its sections folded as given, in a window tall enough for all of it.
fn drawn(state: &State, sections: SideSections) -> TreeUpdate {
    let data = ViewData::new(state, Vec::new(), NOW);
    let window_state = data.window_state(state);
    let side = window_state.side.unwrap();
    let column = SideColumnState { sections, ..side };
    let mut window = Window::new(ThemeName::Dark, (240, 1600));
    window.frame(&[], |ui| side_column_view(&column, ui)).1
}

/// How high the side column of a state wants to be with **Changes** folded: **Repositories** with **Branches** folded,
/// to the foot of its last row, and **Branches** with **Repositories** folded, from under the heading of
/// **Repositories**, the heading of **Changes** counted, to the foot of the last row of the rail.
fn wanted_height(state: &State) -> f32 {
    let unfolded = drawn(
        state,
        SideSections {
            is_changes_folded: true,
            is_branches_folded: true,
            ..SideSections::default()
        },
    );
    let column = bounds(&unfolded, Role::Navigation, words::SIDE_COLUMN);
    let repositories = lowest_in(&unfolded, words::TABS) - column.y;
    let folded = drawn(
        state,
        SideSections {
            is_repositories_folded: true,
            is_changes_folded: true,
            ..SideSections::default()
        },
    );
    let heading = bounds(&folded, Role::Button, words::REPOSITORIES);
    repositories + lowest_in(&folded, views::text::rail::RAIL) - heading.bottom()
}

#[test]
fn repo_side_column__a_repository_with_three_worktrees_and_two_local_branches_takes_less_height_than_before_phase_14() {
    let (_kept, state) = three_worktrees();
    let height = wanted_height(&state);
    assert!(height < BEFORE_PHASE_14, "{height} against {BEFORE_PHASE_14}");
}

/// A repository open, made by hand: its name, and what it is on, `<branch> ● 3 ↓2`.
fn long_entry(name: &str, branch: &str) -> TabEntry {
    TabEntry {
        name: name.to_owned(),
        repository: name.to_owned(),
        place: None,
        folder: format!("/work/{name}"),
        is_shown: true,
        is_running: false,
        summary: Some(TabSummary {
            summary: WorktreeSummary {
                head: SummaryHead::Branch(branch.to_owned()),
                changed: 3,
                upstream: Some((0, 2)),
            },
            is_stale: false,
        }),
        members: Vec::new(),
        is_unfolded: true,
        remembers_resolutions: None,
    }
}

/// The rows of a repository drawn alone in a section of a width, and the tree.
fn rows_drawn(tabs: &[TabEntry], width: u32) -> TreeUpdate {
    let state = TabsState { tabs, not_open: &[] };
    let mut window = Window::new(ThemeName::Dark, (width, 120));
    window.frame(&[], |ui| tabs_view(&state, ui)).1
}

/// The label of some words, which the test fails without.
fn label(tree: &TreeUpdate, words: &str) -> Rect {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Label && node.value() == Some(words))
        .find_map(|(_, node)| node.bounds().map(rect))
        .unwrap_or_else(|| panic!("no label {words}"))
}

#[test]
fn repo_tabs__a_row_is_one_line_26_high_and_in_a_narrow_section_its_name_and_branch_are_both_cut() {
    let short = [long_entry("web", "main")];
    let tree = rows_drawn(&short, 240);
    let row = bounds(&tree, Role::Button, "web");
    assert!((row.height - 26.0).abs() < 0.5, "{row:?}");
    let (name, branch, distance) = (label(&tree, "web"), label(&tree, "main"), label(&tree, "\u{2193}2"));
    assert!(
        name.y >= row.y && name.bottom() <= row.bottom() + 0.5,
        "on the one line"
    );
    assert!(
        branch.x > name.right() && distance.x > branch.right(),
        "after it, on its right"
    );
    assert!(
        row.right() - distance.right() < 40.0,
        "at the right of the row, before its ×"
    );
    let name = "the-repository-of-the-payments-service";
    let branch = "feature/a-very-long-branch-for-the-refunds";
    let long = [long_entry(name, branch)];
    let wide = label(&rows_drawn(&long, 900), branch);
    let tree = rows_drawn(&long, 200);
    let cut_name = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Label)
        .find_map(|(_, node)| {
            let words = node.value()?;
            (words.starts_with('\u{2026}') && name.ends_with(&words['\u{2026}'.len_utf8()..]))
                .then(|| rect(node.bounds().unwrap()))
        })
        .expect("the name cut at its start");
    let row = bounds(&tree, Role::Button, name);
    let room = row.width - 16.0;
    assert!(
        cut_name.width >= room * 0.5 - 24.0,
        "the name keeps half the row: {cut_name:?} {row:?}"
    );
    let narrow = label(&tree, branch);
    assert!(narrow.width < wide.width, "the branch is cut: {narrow:?} {wide:?}");
    let _ = label(&tree, "\u{2193}2");
}

#[test]
fn repo_side_column__the_heading_of_branches_has_create_branch_compare_and_the_filter_on_its_line_each_24_high() {
    let (_kept, state) = three_worktrees();
    let (_, tree) = Window::new(ThemeName::Dark, (1000, 580)).draw(&state, &[]);
    let heading = bounds(&tree, Role::Button, words::BRANCHES);
    let filter = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button)
        .find_map(|(_, node)| {
            let name = node.label()?;
            name.starts_with("Branches listed in the branch rail")
                .then(|| rect(node.bounds().unwrap()))
        })
        .expect("the button of the rail's filter");
    let in_heading = |name: &str| {
        tree.nodes
            .iter()
            .filter(|(_, node)| node.role() == Role::Button && node.label() == Some(name))
            .filter_map(|(_, node)| node.bounds().map(rect))
            .find(|found| found.x >= heading.x)
            .unwrap_or_else(|| panic!("no {name} in the side column"))
    };
    let create = in_heading(views::text::rail::CREATE_BRANCH);
    let compare = in_heading(views::text::rail::COMPARE);
    for button in [create, compare, filter] {
        assert!(button.height >= 24.0, "{button:?}");
        assert!(
            button.y >= heading.y - 0.5 && button.bottom() <= heading.bottom() + 0.5,
            "on the line of the heading: {button:?} {heading:?}"
        );
    }
    assert!(
        create.right() <= compare.x && compare.right() <= filter.x,
        "in that order"
    );
}
