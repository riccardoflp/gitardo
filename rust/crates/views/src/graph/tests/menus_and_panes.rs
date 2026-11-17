//! The menus of the draft, a stash and a merge, the fold button, the panes and the labels.

use model::contract::{FeatureId, Target};
use ui::Rect;

use super::super::commit_menu::double_click_label;
use super::super::draft::{DraftEntry, draft_entries};
use super::super::fold::{MergeEntry, merge_menu};
use super::super::panes::{PaneLayout, PaneSizes, pane_rects};
use super::super::state::DraftState;
use super::{Fixture, feature_and_main, id, repository, rows};
use crate::feature::FeatureIntent;
use crate::graph::StashEntry;
use crate::intent::Intent;

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
        gap: 1.0,
        bottom: 0.0,
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
fn graph_view__narrowing_the_window_puts_the_side_column_away_and_the_inspector_over_the_commits() {
    let place = crate::SidePlace::default();
    assert!(place.is_shown(false), "beside the commits in a wide window");
    assert!(!place.is_shown(true), "put away in a narrow one");
    let layout = PaneLayout {
        is_rail_hidden: !place.is_shown(true),
        is_inspector_over: true,
        ..panes(650.0, PaneSizes::default())
    };
    let rects = pane_rects(&layout, 1.0);
    assert_eq!(rects.rail, None);
    assert_eq!(rects.main.x, 0.0);
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

#[test]
fn graph_view__a_row_keeps_room_for_its_message_and_counts_the_labels_that_do_not_fit() {
    use model::graph::{Label, LabelKind};

    use super::super::row::fitted_pills;

    let labels: Vec<Label> = ["main", "feature", "origin/main", "origin/feature", "v1.0"]
        .iter()
        .map(|name| Label {
            name: (*name).to_owned(),
            kind: LabelKind::Tag,
            is_checked_out: false,
        })
        .collect();
    let width_of = |pill: &super::super::row::Pill<'_>| {
        10.0 * f32::from(u8::try_from(pill.text.chars().count()).unwrap_or(u8::MAX)) + 20.0
    };
    let words =
        |pills: &[super::super::row::Pill<'_>]| pills.iter().map(|pill| pill.text.to_string()).collect::<Vec<_>>();

    let (all, width) = fitted_pills(&labels, 1_000.0, false, width_of);
    assert_eq!(
        words(&all),
        ["main", "feature", "origin/main", "origin/feature", "v1.0"]
    );
    assert!((width - 500.0).abs() < 0.01);

    let (some, width) = fitted_pills(&labels, 200.0, false, width_of);
    assert_eq!(words(&some), ["main", "feature", "+3"]);
    assert!(width <= 200.0);
    let tooltip = some.last().and_then(|pill| pill.tooltip.clone());
    assert_eq!(tooltip.as_deref(), Some("origin/main, origin/feature, v1.0"));

    let (none, width) = fitted_pills(&labels, 30.0, false, width_of);
    assert!(none.is_empty(), "not even the count fits");
    assert!(width.abs() < 0.01);
}

#[test]
fn graph_view__a_double_click_on_the_label_of_a_branch_not_checked_out_checks_it_out() {
    let fixture = feature_and_main();
    let state = fixture.history();
    let label = |commit: &str, name: &str| {
        let labels = fixture.layout.labels.get(&id(commit));
        let index = labels.and_then(|labels| labels.iter().position(|label| label.name == name));
        (
            id(commit),
            index.unwrap_or_else(|| panic!("{commit} has no label {name}")),
        )
    };
    let check_out = Intent::Feature(FeatureIntent::Run(
        FeatureId("branch.checkout"),
        Target::Branch(String::from("feature")),
    ));
    assert_eq!(
        double_click_label(&state, Some(&label("F2", "feature"))),
        Some(vec![check_out])
    );
    assert_eq!(
        double_click_label(&state, Some(&label("M3", "main"))),
        Some(Vec::new()),
        "the branch checked out does nothing, and opens no page"
    );
    assert_eq!(double_click_label(&state, None), None, "off the labels, the row opens");
}

#[test]
fn changes_stage_all__the_menu_of_the_draft_has_no_stage_all_while_a_file_is_in_conflict() {
    let conflict = DraftState {
        files: 2,
        unstaged: 1,
        conflicted: 1,
        has_commit: true,
        ..DraftState::default()
    };
    assert!(!draft_entries(&conflict).contains(&DraftEntry::StageAll));
    let calm = DraftState {
        conflicted: 0,
        ..conflict
    };
    assert!(draft_entries(&calm).contains(&DraftEntry::StageAll));
}

#[test]
fn changes_discard_all__the_entry_of_the_draft_is_not_in_its_menu_while_a_merge_is_stopped() {
    let merging = DraftState {
        files: 1,
        conflicted: 1,
        has_commit: true,
        is_operation_stopped: true,
        ..DraftState::default()
    };
    assert!(!draft_entries(&merging).contains(&DraftEntry::DiscardAll));
}

#[test]
fn the_entries_of_the_draft_run_their_operations_on_the_whole_working_tree() {
    use crate::graph::{draft_entry_intent, draft_menu};
    let draft = DraftState {
        files: 2,
        staged: 1,
        unstaged: 1,
        has_commit: true,
        ..DraftState::default()
    };
    let enabled: Vec<(DraftEntry, bool)> = draft_menu(&draft, false)
        .iter()
        .filter_map(|entry| match entry {
            ui::MenuEntry::Item(item) => Some((item.value, item.availability == ui::Availability::Enabled)),
            _ => None,
        })
        .collect();
    assert_eq!(
        enabled,
        [
            (DraftEntry::Stash, true),
            (DraftEntry::StashStaged, true),
            (DraftEntry::StageAll, true),
            (DraftEntry::UnstageAll, true),
            (DraftEntry::DiscardAll, true),
        ]
    );
    assert_eq!(
        draft_entry_intent(DraftEntry::DiscardAll),
        Some(crate::Intent::Feature(crate::feature::FeatureIntent::Run(
            model::contract::FeatureId("changes.discard-all"),
            model::contract::Target::Head,
        )))
    );
    assert!(
        draft_menu(&draft, true).iter().all(|entry| match entry {
            ui::MenuEntry::Item(item) => item.availability == ui::Availability::Disabled,
            _ => true,
        }),
        "nothing while an operation runs"
    );
}

#[test]
fn stash_in_graph__the_context_menu_of_a_stash_row_has_the_four_stash_entries_only_each_running_its_operation() {
    let items: Vec<(StashEntry, bool)> = super::super::stashes::stash_menu()
        .iter()
        .filter_map(|entry| match entry {
            ui::MenuEntry::Item(item) => Some((item.value, item.availability == ui::Availability::Enabled)),
            _ => None,
        })
        .collect();
    assert_eq!(
        items,
        [
            (StashEntry::Apply, true),
            (StashEntry::Pop, true),
            (StashEntry::CreateBranch, true),
            (StashEntry::Drop, true)
        ]
    );
    let run = |entry| match super::super::stashes::stash_intent(entry, 2) {
        crate::Intent::Feature(crate::feature::FeatureIntent::Run(feature, model::contract::Target::Stash(2))) => {
            feature.0
        }
        _ => "",
    };
    assert_eq!(
        [
            StashEntry::Apply,
            StashEntry::Pop,
            StashEntry::CreateBranch,
            StashEntry::Drop
        ]
        .map(run),
        ["stash.apply", "stash.pop", "stash.branch", "stash.drop"]
    );
}

#[test]
fn graph_combine_upstream__the_menu_of_a_shared_label_ends_with_copy_of_the_upstream() {
    use model::graph::{Label, LabelKind};
    let label = |upstream: Option<&str>| Label {
        name: String::from("feature"),
        kind: LabelKind::Branch {
            worktree: None,
            upstream: upstream.map(str::to_owned),
        },
        is_checked_out: false,
    };
    assert_eq!(
        super::super::commit_menu::copy_upstream(&label(Some("origin/feature"))),
        Some((String::from("Copy origin/feature"), "origin/feature"))
    );
    assert_eq!(super::super::commit_menu::copy_upstream(&label(None)), None);
}
