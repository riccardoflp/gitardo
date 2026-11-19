//! The side column of repositories and branches (`repo.side-column`), its **Repositories** (`repo.tabs`) and the line
//! between its sections (`graph.resize-panes`), over clones made for each test.

use std::path::{Path, PathBuf};

use app::store::{Message, State, ViewData};
use platform::accesskit::{Role, TreeUpdate};
use render::{Point, Rect, ThemeName};
use testkit::Repository;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey, WheelDelta, WheelEvent};
use views::Intent;
use views::graph::{GraphIntent, SideSection};
use views::page::Page;
use views::repo::{
    SideColumnState, SideSections, TabEntry, TabSummary, TabsState, counts_words, head_words, side_column_view,
    tabs_view,
};
use views::text::{header, repo as words};

use super::inspector::ask;
use super::pointer::{apply, bounds, drag, moved, rect};
use super::support::{self, NOW, Window, has, says};
use super::tabs::{WIDE, clone_three, three_clones};
use super::{repository_with_history, showing};

/// The height of the header: a button and its margins.
pub(super) const HEADER: f32 = 48.0;
/// The height of the status bar.
const STATUS: f32 = 26.0;
/// The thickness of the line above the heading of a section.
const LINE: f32 = 1.0;

/// The side column drawn.
pub(super) fn column(tree: &TreeUpdate) -> Rect {
    bounds(tree, Role::Navigation, words::SIDE_COLUMN)
}

/// The rows of **Repositories** drawn.
fn rows(tree: &TreeUpdate) -> Rect {
    bounds(tree, Role::Navigation, words::TABS)
}

/// A node of the header, by its role and its name: one that is above the side column.
pub(super) fn in_header(tree: &TreeUpdate, role: Role, name: &str) -> Rect {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == role && (node.label() == Some(name) || node.value() == Some(name)))
        .filter_map(|(_, node)| node.bounds().map(rect))
        .find(|bounds| bounds.bottom() <= HEADER)
        .unwrap_or_else(|| panic!("no {name} in the header among {:?}", header_nodes(tree)))
}

/// The roles and the names of what the header has.
pub(super) fn header_nodes(tree: &TreeUpdate) -> Vec<String> {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.bounds().is_some_and(|bounds| bounds.y1 <= f64::from(HEADER)))
        .map(|(_, node)| format!("{:?} {:?}", node.role(), node.label().or(node.value())))
        .collect()
}

/// A button of the header, by its name.
pub(super) fn header_button(tree: &TreeUpdate, name: &str) -> Rect {
    in_header(tree, Role::Button, name)
}

/// **Sync** in the header, whatever counts it has.
pub(super) fn sync(tree: &TreeUpdate) -> (String, Rect) {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button)
        .find_map(|(_, node)| {
            let label = node.label().filter(|label| label.ends_with("Sync"))?;
            Some((label.to_owned(), rect(node.bounds()?)))
        })
        .unwrap_or_else(|| panic!("no Sync"))
}

/// The names of the entries of the menu open.
pub(super) fn menu_items(tree: &TreeUpdate) -> Vec<String> {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::MenuItem)
        .filter_map(|(_, node)| node.label())
        .map(str::to_owned)
        .collect()
}

/// The window opened again on the three clones, with what it kept.
fn opened_again(state: &State, clones: &[PathBuf; 3]) -> State {
    let mut again = super::window();
    again.persisted = state.persisted.clone();
    let _ = support::run(
        &mut again,
        Message::Started {
            folders: clones.to_vec(),
        },
    );
    again
}

/// Runs git in a clone.
pub(super) fn git_in(origin: &Repository, folder: &Path, arguments: &[&str]) {
    let mut all = vec!["-C", folder.to_str().unwrap()];
    all.extend_from_slice(arguments);
    let _ = origin.git(&all);
}

/// A repository open, made by hand, with a summary of `main` and nothing to say.
fn entry(name: &str, is_shown: bool) -> TabEntry {
    TabEntry {
        name: name.to_owned(),
        repository: name.to_owned(),
        place: None,
        folder: format!("/work/{name}"),
        is_shown,
        is_running: false,
        summary: Some(TabSummary {
            summary: model::repo::WorktreeSummary {
                head: model::repo::SummaryHead::Branch(String::from("main")),
                ..model::repo::WorktreeSummary::default()
            },
            is_stale: false,
        }),
        members: Vec::new(),
        looked_after: None,
        is_unfolded: true,
        remembers_resolutions: None,
    }
}

#[test]
fn repo_side_column__its_top_is_under_the_header_level_with_the_timeline_and_its_foot_touches_the_status_bar() {
    let (_kept, _clones, state) = three_clones();
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    let column = column(&tree);
    let timeline = bounds(&tree, Role::Main, "Timeline");
    let top_row = bounds(&tree, Role::Button, header::HIDE_REFERENCES);
    assert!(column.y >= HEADER, "{column:?} is under the header");
    assert!(
        top_row.y > column.y && top_row.y - column.y <= 12.0,
        "the top row of the timeline is level with it: {column:?} {top_row:?}"
    );
    #[allow(clippy::cast_precision_loss, reason = "the window is a few hundred pixels high")]
    let status = WIDE.1 as f32 - STATUS;
    assert!(
        (column.bottom() - timeline.bottom()).abs() < 0.5,
        "{column:?} {timeline:?}"
    );
    assert!(
        column.bottom() <= status && status - column.bottom() <= 8.0,
        "{column:?}"
    );
    assert!(column.right() < timeline.x, "the timeline is on its right");
}

#[test]
fn repo_side_column__folding_changes_and_branches_gives_repositories_the_whole_height_but_their_headings() {
    let (_kept, _clones, mut state) = three_clones();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, words::BRANCHES);
    assert_eq!(
        intents,
        [Intent::Graph(GraphIntent::FoldSection(SideSection::Branches))]
    );
    apply(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    let branches = bounds(&tree, Role::Button, words::BRANCHES);
    assert!((branches.bottom() - column(&tree).bottom()).abs() < 0.5, "{branches:?}");
    assert!(!has(&tree, Role::Navigation, views::text::rail::RAIL));
    // **Changes**, the last section open, has the height **Branches** left.
    let changes = super::pointer::changes_section(&tree).unwrap();
    assert!((branches.y - changes.bottom() - LINE).abs() < 0.5, "{changes:?}");
    let intents = window.click(&state, Role::Button, words::CHANGES);
    assert_eq!(intents, [Intent::Graph(GraphIntent::FoldSection(SideSection::Changes))]);
    apply(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    let column = column(&tree);
    let changes = bounds(&tree, Role::Button, words::CHANGES);
    let branches = bounds(&tree, Role::Button, words::BRANCHES);
    assert!(
        (branches.bottom() - column.bottom()).abs() < 0.5,
        "{branches:?} {column:?}"
    );
    // Each heading has its line above it: the rows, a line, **Changes**, a line, **Branches**.
    assert!(
        (branches.y - changes.bottom() - LINE).abs() < 0.5,
        "{changes:?} {branches:?}"
    );
    assert!((changes.y - rows(&tree).bottom() - LINE).abs() < 0.5);
    assert_eq!(super::pointer::changes_section(&tree), None);
}

#[test]
fn repo_side_column__folding_all_three_leaves_three_headings_at_the_top_a_line_between_them_and_the_rest_empty() {
    let (_kept, _clones, mut state) = three_clones();
    let folds = [SideSection::Repositories, SideSection::Changes, SideSection::Branches]
        .map(|section| Intent::Graph(GraphIntent::FoldSection(section)));
    apply(&mut state, folds.to_vec());
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    let column = column(&tree);
    let first = bounds(&tree, Role::Button, words::REPOSITORIES);
    let second = bounds(&tree, Role::Button, words::CHANGES);
    let third = bounds(&tree, Role::Button, words::BRANCHES);
    assert!((first.y - column.y).abs() < 0.5);
    assert!(
        (second.y - first.bottom() - LINE).abs() < 0.5,
        "a line apart: {first:?} {second:?}"
    );
    assert!(
        (third.y - second.bottom() - LINE).abs() < 0.5,
        "a line apart: {second:?} {third:?}"
    );
    assert!(!has(&tree, Role::Navigation, words::TABS));
    assert_eq!(super::pointer::changes_section(&tree), None);
    assert!(!has(&tree, Role::Navigation, views::text::rail::RAIL));
}

#[test]
fn repo_side_column__the_line_above_the_headings_of_changes_and_of_branches_is_drawn_whatever_is_folded() {
    let border = render::Theme::new(ThemeName::Dark, render::Appearance::Dark)
        .colors
        .border;
    let sections = [SideSection::Repositories, SideSection::Changes, SideSection::Branches];
    for bits in 0..8_u8 {
        let folded: Vec<SideSection> = sections
            .into_iter()
            .enumerate()
            .filter(|(index, _)| bits & (1 << index) != 0)
            .map(|(_, section)| section)
            .collect();
        let (_kept, _clones, mut state) = three_clones();
        let folds = folded
            .iter()
            .map(|section| Intent::Graph(GraphIntent::FoldSection(*section)));
        apply(&mut state, folds.collect());
        let mut window = Window::new(ThemeName::Dark, WIDE);
        let (_, tree) = window.draw(&state, &[]);
        for name in [words::CHANGES, words::BRANCHES] {
            let heading = bounds(&tree, Role::Button, name);
            let middle = heading.x + heading.width * 0.5;
            #[allow(
                clippy::cast_possible_truncation,
                reason = "a window is far smaller than an i32 counts"
            )]
            let pixel = |y: f32| window.frame.pixel(middle as i32, y.floor() as i32);
            assert_eq!(
                pixel(heading.y - LINE),
                Some(border),
                "the line above the heading of {name}: {folded:?}"
            );
            assert_ne!(pixel(heading.y - LINE - 2.0), Some(border), "{name}: {folded:?}");
            assert_ne!(pixel(heading.y + 2.0), Some(border), "{name}: {folded:?}");
        }
    }
}

#[test]
fn repo_side_column__dragging_the_line_between_the_sections_changes_their_heights_and_they_are_kept() {
    let (_kept, clones, mut state) = three_clones();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let before = rows(&tree);
    let line = Point::new(before.x + before.width * 0.5, before.bottom() + 0.5);
    let intents = drag(&mut window, &state, line, Point::new(line.x, line.y + 40.0));
    apply(&mut state, intents);
    let again = opened_again(&state, &clones);
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&again, &[]);
    assert!(
        (rows(&tree).height - before.height - 40.0).abs() < 0.5,
        "{:?}",
        rows(&tree)
    );
}

#[test]
fn repo_side_column__it_stays_beside_the_changes_page_and_the_settings_page() {
    let (_kept, _clones, mut state) = three_clones();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let beside_timeline = column(&tree);
    for page in [Page::Changes, Page::Settings] {
        ask(&mut state, Intent::ShowPage(page));
        let (_, tree) = window.draw(&state, &[]);
        assert_eq!(column(&tree), beside_timeline, "{page:?}");
    }
}

#[test]
fn repo_side_column__in_a_window_800_wide_it_is_put_away_and_the_first_button_of_the_timeline_opens_it_over() {
    let (_kept, _clones, mut state) = three_clones();
    let mut window = Window::new(ThemeName::Dark, (800, 580));
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::Navigation, words::SIDE_COLUMN));
    let timeline = bounds(&tree, Role::Main, "Timeline");
    let toggle = bounds(&tree, Role::Button, header::SHOW_REFERENCES);
    assert!(
        toggle.y < timeline.y + 48.0 && toggle.x < timeline.x + 48.0,
        "the first button of its top row"
    );
    let intents = window.click(&state, Role::Button, header::SHOW_REFERENCES);
    assert_eq!(intents, [Intent::Graph(GraphIntent::OpenSideColumn)]);
    apply(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    let over = column(&tree);
    assert!(
        over.x < timeline.x + 8.0 && over.right() > timeline.x,
        "over the timeline: {over:?}"
    );
    let escape = InputEvent::KeyPressed(KeyEvent::press(Key::Named(NamedKey::Escape), Modifiers::NONE));
    let (intents, _) = window.draw(&state, &[escape]);
    assert_eq!(intents, [Intent::Graph(GraphIntent::CloseSideColumn)]);
}

#[test]
fn graph_resize_panes__dragging_the_line_under_repositories_down_gives_it_the_height_branches_loses() {
    let (_kept, _clones, mut state) = three_clones();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let (above, under) = (rows(&tree), bounds(&tree, Role::Navigation, views::text::rail::RAIL));
    let line = Point::new(above.x + above.width * 0.5, above.bottom() + 0.5);
    let intents = drag(&mut window, &state, line, Point::new(line.x, line.y + 30.0));
    apply(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    let (taller, shorter) = (rows(&tree), bounds(&tree, Role::Navigation, views::text::rail::RAIL));
    assert!((taller.height - above.height - 30.0).abs() < 0.5, "{taller:?}");
    assert!((under.height - shorter.height - 30.0).abs() < 0.5, "{shorter:?}");
}

#[test]
fn repo_tabs__web_on_main_with_3_files_changed_and_2_commits_to_pull_says_main_3_2() {
    let ((mut origin, holder), clones) = clone_three();
    let _ = origin.commit("Pull the first", &[("pull.txt", "1\n")]);
    let _ = origin.commit("Pull the second", &[("pull.txt", "2\n")]);
    git_in(&origin, &clones[1], &["fetch", "--quiet"]);
    for name in ["one.txt", "two.txt", "three.txt"] {
        std::fs::write(clones[1].join(name), "changed\n").unwrap();
    }
    let mut state = super::window();
    let _ = support::run(
        &mut state,
        Message::Started {
            folders: clones.to_vec(),
        },
    );
    ask(&mut state, Intent::SwitchTab(1));
    let data = ViewData::new(&state, Vec::new(), NOW);
    let side = data.window_state(&state).side.unwrap();
    let summary = side.tabs.tabs[1].summary.clone().unwrap();
    let said = format!("{} {}", head_words(&summary.summary.head), counts_words(&summary));
    assert_eq!(said, "main \u{25cf} 3 \u{2193}2");
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    assert!(says(&tree, "\u{2193}2"));
    drop(holder);
}

#[test]
fn repo_tabs__api_on_main_up_to_date_and_with_nothing_changed_says_main_alone() {
    let (_kept, _clones, state) = three_clones();
    let data = ViewData::new(&state, Vec::new(), NOW);
    let side = data.window_state(&state).side.unwrap();
    let summary = side.tabs.tabs[0].summary.clone().unwrap();
    assert_eq!(head_words(&summary.summary.head), "main");
    assert_eq!(counts_words(&summary), "");
}

#[test]
fn repo_tabs__with_twelve_repositories_in_a_section_200_high_it_scrolls_to_the_last_one() {
    let tabs: Vec<TabEntry> = (1..=12)
        .map(|number| entry(&format!("repository {number}"), number == 1))
        .collect();
    let state = TabsState {
        tabs: &tabs,
        not_open: &[],
    };
    let mut window = Window::new(ThemeName::Dark, (240, 200));
    let middle = Point::new(120.0, 100.0);
    let last = |tree: &TreeUpdate| bounds(tree, Role::Button, "repository 12");
    let (_, tree) = window.frame(&[moved(middle)], |ui| tabs_view(&state, ui));
    assert!(
        !has(&tree, Role::Button, "repository 12") || last(&tree).bottom() > 200.0,
        "the last one is out of sight at first"
    );
    let turn = InputEvent::Wheel(WheelEvent {
        delta: WheelDelta::Lines { x: 0.0, y: 40.0 },
        position: middle,
        modifiers: Modifiers::NONE,
    });
    let _ = window.frame(&[turn], |ui| tabs_view(&state, ui));
    window.now += std::time::Duration::from_secs(1);
    let (_, tree) = window.frame(&[], |ui| tabs_view(&state, ui));
    let shown = last(&tree);
    assert!(shown.y >= 0.0 && shown.bottom() <= 200.5, "{shown:?}");
}

#[test]
fn repo_tabs__there_is_no_island_above_the_header() {
    let (_kept, _clones, state) = three_clones();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let repository = header_button(&tree, "api");
    assert!(repository.y >= 0.0 && repository.bottom() <= HEADER);
    assert!(
        rows(&tree).y > HEADER,
        "the rows are in the side column, under the header"
    );
    let pixels = window.frame.to_rgba();
    let ground = |x: usize, y: usize| {
        let at = (y * WIDE.0 as usize + x) * 4;
        pixels[at..at + 3].to_vec()
    };
    let column = column(&tree);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "inside the window"
    )]
    let x = (column.x + 4.0) as usize;
    assert_eq!(ground(x, 1), ground(x, 46), "the header is on the ground");
    assert_eq!(ground(x, 1), [0x0E, 0x0F, 0x12], "the ground of the dark theme");
}

#[test]
fn repo_tabs__the_snapshots_of_the_section_with_one_repository_with_several_and_with_worktrees_match_in_every_theme_at_both_scales()
 {
    let (_one_kept, one_folder) = {
        let (repository, remote) = repository_with_history();
        let folder = repository.path().to_path_buf();
        ((repository, remote), folder)
    };
    let one = showing(one_folder);
    let (_kept, _clones, mut several) = three_clones();
    ask(&mut several, Intent::SwitchTab(1));
    let grouped = super::group_tabs::made(false);
    let mut worktrees = super::group_tabs::window(&grouped);
    ask(&mut worktrees, Intent::ShowInTab(0, grouped.login.clone()));
    let themes = [
        (ThemeName::Dark, "dark"),
        (ThemeName::Light, "light"),
        (ThemeName::HighContrastDark, "high-contrast-dark"),
        (ThemeName::HighContrastLight, "high-contrast-light"),
    ];
    for (theme, theme_name) in themes {
        for scale in [1, 2] {
            for (state, layout) in [(&one, "one"), (&several, "several"), (&worktrees, "worktrees")] {
                let mut window = Window::scaled(theme, (240, 220), scale);
                let data = ViewData::new(state, Vec::new(), NOW);
                let window_state = data.window_state(state);
                let side = window_state.side.unwrap();
                let section = SideColumnState {
                    tabs: side.tabs,
                    changes: side.changes,
                    branches: None,
                    sections: SideSections {
                        is_changes_folded: true,
                        is_branches_folded: true,
                        ..SideSections::default()
                    },
                };
                let _ = window.frame(&[], |ui| side_column_view(&section, ui));
                let suffix = if scale == 2 { "-2x" } else { "" };
                window.assert_snapshot(&format!("repositories-{layout}-{theme_name}{suffix}"));
            }
        }
    }
}
