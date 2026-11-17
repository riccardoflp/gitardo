//! The entries of the menu of the view of the header, in the groups and the order of its card (`graph.view-menu`).

use model::filter::{Kinds, ReferenceKind};
use ui::MenuEntry;

use super::{GraphDrawn, TimelineMenu, ViewCommand, menu_entries};
use crate::graph::intent::{Switches, TimelineSwitch};

/// The words of the menu, with `-` for a separator.
fn words(menu: TimelineMenu) -> Vec<&'static str> {
    menu_entries(menu)
        .into_iter()
        .map(|entry| match entry {
            MenuEntry::Item(item) => item.label,
            MenuEntry::Submenu { .. } | MenuEntry::Separator => "-",
        })
        .collect()
}

/// The words of the whole menu, in the groups and the order of the card.
const IN_ORDER: [&str; 17] = [
    "Show local branches",
    "Show remote branches",
    "Show tags",
    "Show stashes",
    "Show only the first parent of merges",
    "-",
    "Show all commits on one line",
    "Compact rows",
    "Bring out the history of the selected commit",
    "Separate the days, weeks and months",
    "Put a branch and its upstream on one label",
    "Fold all merges",
    "Unfold all merges",
    "-",
    "Show the author",
    "Show the date",
    "Show the hash",
];

/// A timeline with its lines, merges loaded and one folded, every kind of reference shown, the switches as they start.
fn timeline() -> TimelineMenu {
    TimelineMenu {
        switches: Switches::DEFAULT,
        has_merges: true,
        has_folds: true,
        is_all_folded: false,
        kinds: Some(Kinds::ALL),
        drawn: GraphDrawn::WithLines,
    }
}

#[test]
fn graph_view_menu__every_switch_is_in_the_menu_in_its_group_and_in_the_order_of_the_card() {
    assert_eq!(words(timeline()), IN_ORDER);
}

#[test]
fn graph_view_menu__each_switch_shows_its_check_while_it_is_on_and_no_other_icon() {
    let switches = Switches::DEFAULT
        .with(TimelineSwitch::Hash, false)
        .with(TimelineSwitch::FirstParent, true);
    let kinds = Kinds::ALL.with(ReferenceKind::Remote, false);
    let menu = TimelineMenu {
        switches,
        kinds: Some(kinds),
        ..timeline()
    };
    for entry in menu_entries(menu) {
        let MenuEntry::Item(item) = entry else {
            continue;
        };
        let is_on = match item.value {
            ViewCommand::Switch(switch) => switches.is_on(switch),
            ViewCommand::Kind(kind) => kinds.has(kind),
            ViewCommand::FoldAll(super::FoldAllEntry::Fold) => menu.is_all_folded,
            ViewCommand::FoldAll(super::FoldAllEntry::Unfold) => continue,
        };
        assert_eq!(item.icon, is_on.then_some(crate::icons::CHOSEN), "{}", item.label);
    }
}

#[test]
fn graph_view_menu__the_folds_are_there_only_while_the_graph_has_its_lines_and_is_not_on_one_line() {
    let with = words(timeline());
    assert!(with.contains(&"Fold all merges") && with.contains(&"Unfold all merges"));
    let listed = words(TimelineMenu {
        drawn: GraphDrawn::AsList,
        ..timeline()
    });
    assert!(!listed.contains(&"Fold all merges") && !listed.contains(&"Unfold all merges"));
    assert_eq!(listed.len(), IN_ORDER.len() - 2, "every switch stays");
}

#[test]
fn graph_one_lane__on_fold_all_merges_is_not_in_the_menu() {
    let on = words(TimelineMenu {
        switches: Switches::DEFAULT.with(TimelineSwitch::OneLane, true),
        ..timeline()
    });
    assert!(on.contains(&"Show all commits on one line"));
    assert!(!on.contains(&"Fold all merges"), "{on:?}");
    assert!(!on.contains(&"Unfold all merges"));
}
