//! The menus of the draft, a stash and a merge, the fold button, the panes and the labels.

use ui::Rect;

use super::super::draft::{DraftEntry, draft_entries};
use super::super::fold::{MergeEntry, merge_menu};
use super::super::panes::{PaneLayout, PaneSizes, pane_rects};
use super::super::state::DraftState;
use super::{Fixture, feature_and_main, id, repository, rows};
use crate::graph::StashEntry;

#[test]
fn graph_uncommitted_row__with_a_file_in_conflict_no_stage_all_and_during_a_merge_no_discard_all() {
    let conflict = DraftState {
        files: 2,
        unstaged: 1,
        conflicted: 1,
        has_commit: true,
        ..DraftState::default()
    };
    assert!(!draft_entries(&conflict).contains(&DraftEntry::StageAll));
    let merging = DraftState {
        is_operation_stopped: true,
        ..conflict
    };
    assert!(!draft_entries(&merging).contains(&DraftEntry::DiscardAll));
    let calm = DraftState {
        conflicted: 0,
        staged: 1,
        ..conflict
    };
    assert_eq!(
        draft_entries(&calm),
        [
            DraftEntry::Stash,
            DraftEntry::StashStaged,
            DraftEntry::StageAll,
            DraftEntry::UnstageAll,
            DraftEntry::DiscardAll
        ]
    );
}

#[test]
fn graph_uncommitted_row__before_the_first_commit_the_menu_has_no_stash_entry() {
    let first = DraftState {
        files: 1,
        unstaged: 1,
        ..DraftState::default()
    };
    let entries = draft_entries(&first);
    assert!(!entries.contains(&DraftEntry::Stash) && !entries.contains(&DraftEntry::StashStaged));
    assert_eq!(entries, [DraftEntry::StageAll, DraftEntry::DiscardAll]);
}

#[test]
fn graph_stashes__the_context_menu_of_a_stash_row_has_apply_pop_drop_and_create_branch_and_nothing_of_a_commit() {
    let values: Vec<StashEntry> = super::super::stashes::stash_menu()
        .iter()
        .filter_map(|entry| match entry {
            ui::MenuEntry::Item(item) => Some(item.value),
            _ => None,
        })
        .collect();
    assert_eq!(
        values,
        [
            StashEntry::Apply,
            StashEntry::Pop,
            StashEntry::CreateBranch,
            StashEntry::Drop
        ]
    );
}

#[test]
fn graph_fold_merge__a_merge_that_brought_nothing_in_has_no_arrow_and_no_entry() {
    assert_eq!(merge_menu(false, false), []);
    assert_eq!(merge_menu(true, false).len(), 1);
    let fixture = Fixture::new(
        rows(&["M: B F", "B: F", "F: A", "A:"]),
        &repository("M", &[("main", "M")]),
        None,
    );
    assert!(!fixture.layout.foldable.contains(&id("M")));
    let entries = merge_menu(fixture.layout.foldable.contains(&id("M")), false);
    assert!(
        entries
            .iter()
            .all(|entry| !matches!(entry, ui::MenuEntry::Item(item) if item.value == MergeEntry::Fold))
    );
}

#[test]
fn graph_first_parent__a_merge_with_the_setting_on_has_no_fold_button() {
    let with_both = Fixture::new(rows(&["M: B F", "F: A", "B: A", "A:"]), &repository("M", &[]), None);
    assert!(with_both.layout.foldable.contains(&id("M")));
    let first_parent = Fixture::new(rows(&["M: B", "B: A", "A:"]), &repository("M", &[]), None);
    assert!(first_parent.layout.foldable.is_empty());
}

/// The panes of a window of a width, with the sizes dragged.
fn panes(width: f32, sizes: PaneSizes) -> PaneLayout {
    PaneLayout {
        body: Rect::new(0.0, 40.0, width, 600.0),
        sizes,
        icon_rail: None,
        is_rail_hidden: false,
        is_inspector_over: false,
        has_inspector: true,
    }
}

#[test]
fn graph_resize_panes__the_branch_rail_cannot_be_narrower_than_120_px_nor_wider_than_35_percent_of_the_window() {
    let narrow = PaneSizes {
        rail: Some(50.0),
        ..PaneSizes::default()
    };
    let wide = PaneSizes {
        rail: Some(900.0),
        ..PaneSizes::default()
    };
    let width = |sizes| pane_rects(&panes(1_200.0, sizes), 1.0).rail.map(|rail| rail.width);
    assert_eq!(width(narrow), Some(120.0));
    assert_eq!(width(wide), Some(420.0));
    assert_eq!(width(PaneSizes::default()), Some(210.0));
}

#[test]
fn graph_resize_panes__the_inspector_starts_at_320_px_and_a_size_larger_than_the_window_allows_is_drawn_at_the_most() {
    let width = |sizes, window| {
        pane_rects(&panes(window, sizes), 1.0)
            .inspector
            .map(|inspector| inspector.width)
    };
    assert_eq!(width(PaneSizes::default(), 1_200.0), Some(320.0));
    let large = PaneSizes {
        inspector: Some(800.0),
        ..PaneSizes::default()
    };
    assert_eq!(width(large, 1_200.0), Some(660.0));
    assert_eq!(width(large, 1_600.0), Some(800.0));
}

#[test]
fn graph_view__narrowing_the_window_shows_the_rail_as_icons_and_the_inspector_over_the_commits() {
    let layout = PaneLayout {
        icon_rail: Some(56.0),
        is_inspector_over: true,
        ..panes(650.0, PaneSizes::default())
    };
    let rects = pane_rects(&layout, 1.0);
    assert_eq!(rects.rail.map(|rail| rail.width), Some(56.0));
    let inspector = rects.inspector.unwrap_or_else(|| panic!("no inspector"));
    assert_eq!(inspector.x, rects.main.x);
    assert_eq!(inspector.width, rects.main.width);
    assert!(inspector.y > rects.main.y && inspector.bottom() <= rects.main.bottom() + 0.5);
    assert!((inspector.height - 300.0).abs() < 1.0, "{inspector:?}");
}

#[test]
fn graph_view__the_label_of_the_branch_checked_out_is_filled_and_the_others_outlined() {
    let fixture = feature_and_main();
    let labels = |name| fixture.layout.labels.get(&id(name)).cloned().unwrap_or_default();
    let fills = |name| {
        labels(name)
            .iter()
            .map(super::super::row::label_fill)
            .collect::<Vec<_>>()
    };
    assert_eq!(fills("M3"), [ui::BadgeFill::Solid]);
    assert_eq!(fills("F2"), [ui::BadgeFill::Outline]);
}
