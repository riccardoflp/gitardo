//! The entries of the **…** menu of the header, in the groups and the order of its card (`graph.more-menu`).

use model::filter::Kinds;
use ui::MenuEntry;

use super::{GraphDrawn, MainRemote, MoreMenuState, OperationsMenu, TimelineMenu, menu_entries};
use crate::graph::intent::Switches;

/// The words of the menu, with `-` for a separator.
fn words(state: &MoreMenuState) -> Vec<&'static str> {
    menu_entries(state)
        .into_iter()
        .map(|entry| match entry {
            MenuEntry::Item(item) => item.label,
            MenuEntry::Submenu { .. } | MenuEntry::Separator => "-",
        })
        .collect()
}

/// The words of the whole menu, in the groups and the order of the card.
const IN_ORDER: [&str; 36] = [
    "Refresh",
    "Branch",
    "Stash",
    "Undo",
    "-",
    "Show local branches",
    "Show remote branches",
    "Show tags",
    "Show stashes",
    "Show only the first parent of merges",
    "Bring out the history of the selected commit",
    "Fold all merges",
    "Unfold all merges",
    "-",
    "Show the author",
    "Show the date",
    "Show the hash",
    "Separate the days, weeks and months",
    "Put a branch and its upstream on one label",
    "-",
    "Compare branches, tags or commits\u{2026}",
    "Go to commit\u{2026}",
    "History of the active file",
    "Reflog",
    "Clean up branches\u{2026}",
    "Apply patch\u{2026}",
    "Manage remotes\u{2026}",
    "Add submodule\u{2026}",
    "Select repository\u{2026}",
    "-",
    "Switch identity\u{2026}",
    "-",
    "Load pull requests from GitHub",
    "-",
    "Settings",
    "Theme\u{2026}",
];

#[test]
fn graph_more_menu__the_entries_built_are_in_the_groups_and_the_order_of_the_card() {
    let state = MoreMenuState {
        can_refresh: true,
        can_select_repository: true,
        timeline: Some(TimelineMenu {
            switches: Switches::DEFAULT,
            has_merges: true,
            has_folds: false,
            is_all_folded: false,
            kinds: Some(Kinds::ALL),
            drawn: GraphDrawn::WithLines,
        }),
        operations: Some(OperationsMenu {
            is_running: false,
            has_commits: true,
            can_stash: true,
        }),
        has_submodules: false,
        main_remote: MainRemote::OnGitHub,
        active_file: None,
    };
    assert_eq!(words(&state), IN_ORDER);
}

#[test]
fn graph_more_menu__add_submodule_is_not_in_the_menu_of_a_repository_with_submodules() {
    let operations = OperationsMenu {
        is_running: false,
        has_commits: true,
        can_stash: false,
    };
    let with = MoreMenuState {
        operations: Some(operations),
        has_submodules: true,
        ..MoreMenuState::default()
    };
    assert!(!words(&with).contains(&"Add submodule\u{2026}"));
    let without = MoreMenuState {
        has_submodules: false,
        ..with
    };
    assert!(words(&without).contains(&"Add submodule\u{2026}"));
}

#[test]
fn graph_more_menu__the_entries_with_a_condition_are_there_only_while_it_holds() {
    let timeline = TimelineMenu {
        switches: Switches::DEFAULT,
        has_merges: true,
        has_folds: true,
        is_all_folded: false,
        kinds: Some(Kinds::ALL),
        drawn: GraphDrawn::WithLines,
    };
    let state = MoreMenuState {
        can_refresh: true,
        can_select_repository: true,
        timeline: Some(timeline),
        operations: Some(OperationsMenu {
            is_running: false,
            has_commits: true,
            can_stash: true,
        }),
        has_submodules: false,
        main_remote: MainRemote::OnGitHub,
        active_file: None,
    };
    let conditional = [
        "Fold all merges",
        "Unfold all merges",
        "Select repository\u{2026}",
        "Load pull requests from GitHub",
    ];
    let all = words(&state);
    for entry in conditional {
        assert!(all.contains(&entry), "{entry} is there while its condition holds");
    }
    let listed = MoreMenuState {
        timeline: Some(TimelineMenu {
            drawn: GraphDrawn::AsList,
            ..timeline
        }),
        can_select_repository: false,
        main_remote: MainRemote::Elsewhere,
        ..state
    };
    let some = words(&listed);
    for entry in conditional {
        assert!(
            !some.contains(&entry),
            "{entry} is not there once its condition does not hold"
        );
    }
    assert!(some.contains(&"History of the active file"));
}
