//! The entries of the **…** menu of the header, in the groups and the order of its card (`graph.more-menu`): the
//! actions alone.

use ui::MenuEntry;

use super::{MainRemote, MoreMenuState, OperationsMenu, menu_entries};
use crate::text::filter as filter_words;
use crate::text::header as header_words;

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
const IN_ORDER: [&str; 22] = [
    "Refresh",
    "Branch",
    "Stash",
    "Undo",
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
    "Check out a pull request\u{2026}",
    "-",
    "Settings",
    "Theme\u{2026}",
];

/// A repository on GitHub with no submodule, in a workspace of several, and nothing running.
fn everything() -> MoreMenuState {
    MoreMenuState {
        can_refresh: true,
        can_select_repository: true,
        operations: Some(OperationsMenu {
            is_running: false,
            has_commits: true,
            can_stash: true,
        }),
        has_submodules: false,
        main_remote: MainRemote::OnGitHub,
        active_file: None,
    }
}

#[test]
fn graph_more_menu__the_entries_built_are_in_the_groups_and_the_order_of_the_card() {
    assert_eq!(words(&everything()), IN_ORDER);
}

#[test]
fn graph_more_menu__no_switch_of_the_timeline_is_in_the_menu() {
    let all = words(&everything());
    for switch in [
        filter_words::SHOW_LOCAL,
        filter_words::SHOW_REMOTE,
        filter_words::SHOW_TAGS,
        header_words::SHOW_STASHES,
        header_words::FIRST_PARENT,
        header_words::ONE_LANE,
        header_words::TRACE_SELECTED,
        header_words::FOLD_ALL,
        header_words::UNFOLD_ALL,
        header_words::SHOW_AUTHOR,
        header_words::SHOW_DATE,
        header_words::SHOW_HASH,
        header_words::DATE_SEPARATORS,
        header_words::COMBINE_UPSTREAM,
    ] {
        assert!(!all.contains(&switch), "{switch} is in the menu of the view");
    }
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
    let state = everything();
    let conditional = ["Select repository\u{2026}", "Load pull requests from GitHub"];
    let all = words(&state);
    for entry in conditional {
        assert!(all.contains(&entry), "{entry} is there while its condition holds");
    }
    let some = words(&MoreMenuState {
        can_select_repository: false,
        main_remote: MainRemote::Elsewhere,
        ..state
    });
    for entry in conditional {
        assert!(
            !some.contains(&entry),
            "{entry} is not there once its condition does not hold"
        );
    }
    assert!(some.contains(&"History of the active file"));
}

#[test]
fn hosting_load_merge_requests__on_github_the_menu_has_no_such_entry_and_on_gitlab_it_has_one() {
    const LOAD: &str = "Load merge requests from GitLab";
    let on_github = words(&everything());
    assert!(!on_github.contains(&LOAD));
    let on_gitlab = words(&MoreMenuState {
        main_remote: MainRemote::OnService(model::hosting::HostingService::GitLab),
        ..everything()
    });
    assert!(on_gitlab.contains(&LOAD));
    assert!(!on_gitlab.contains(&"Load pull requests from GitHub"));
    let on_gitea = words(&MoreMenuState {
        main_remote: MainRemote::OnService(model::hosting::HostingService::Gitea),
        ..everything()
    });
    assert!(
        !on_gitea.contains(&LOAD),
        "only a GitLab has merge requests Gitardo reads"
    );
    assert!(on_gitea.contains(&"Check out a pull request\u{2026}"));
    assert!(on_gitlab.contains(&"Check out a merge request\u{2026}"));
}

#[test]
fn hosting_checkout_pull_request__on_bitbucket_neither_entry_is_offered() {
    let on_bitbucket = words(&MoreMenuState {
        main_remote: MainRemote::OnService(model::hosting::HostingService::Bitbucket),
        ..everything()
    });
    assert!(
        !on_bitbucket.iter().any(|word| word.starts_with("Check out a")),
        "{on_bitbucket:?}"
    );
    assert!(
        !on_bitbucket.iter().any(|word| word.starts_with("Load")),
        "{on_bitbucket:?}"
    );
    assert_eq!(
        on_bitbucket.iter().filter(|word| **word == "-").count(),
        3,
        "no line of an empty group"
    );
    let label = crate::text::github::check_out_this_pull_request(model::hosting::HostingService::GitLab, 12);
    assert_eq!(label, "Check out merge request !12");
}
