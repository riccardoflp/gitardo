//! The rows of the branch rail made from references made by hand: the lists and their order, the filter box, the line
//! of the branches shown, the favorites, the menus, and what a click asks.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::collections::HashMap;
use std::fmt::Write as _;

use model::filter::{BranchesShown, rank_names};
use model::refs::{References, SortOrder};
use model::state::{Branch, RemoteBranch, RepositoryState, Stash, Tag, Upstream};
use model::{CommitId, Timestamp};
use ui::MenuEntry;

use super::entry::{branch_stands, click_intents};
use super::menu::{EntryCommand, entry_menu};
use super::rows::{Entry, RailRow, is_filter_offered, rail_rows};
use super::{Favorites, ReferencesState};
use crate::filter::FilterEntry;
use crate::graph::GraphIntent;
use crate::intent::{FilterIntent, Intent, RailIntent, RailList, RailLists};

/// The hash of a commit named in a test: its name in hexadecimal, padded.
fn id(name: &str) -> CommitId {
    let hex = name.bytes().take(20).fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    });
    CommitId::parse(&format!("{hex:0<40}")).unwrap_or_else(|| panic!("{name} makes no hash"))
}

fn branch(name: &str) -> Branch {
    Branch {
        name: name.to_owned(),
        commit: id(name),
        upstream: None,
        is_checked_out: name == "main",
        date: Timestamp(0),
    }
}

fn remote(name: &str) -> RemoteBranch {
    RemoteBranch {
        name: name.to_owned(),
        remote: name.split('/').next().unwrap_or_default().to_owned(),
        commit: id(name),
        date: Timestamp(0),
    }
}

fn tag(name: &str) -> Tag {
    Tag {
        name: name.to_owned(),
        commit: id(name),
        date: Timestamp(0),
    }
}

/// Everything the rail is drawn from, owned.
struct Rail {
    repository: RepositoryState,
    references: References,
    lists: RailLists,
    colors: HashMap<String, usize>,
    favorites: Vec<String>,
    recent: Vec<String>,
    typed: String,
    shown: Option<BranchesShown>,
    follows: bool,
}

impl Rail {
    fn new(branches: &[&str], remotes: &[&str], tags: &[&str]) -> Rail {
        let references = References {
            branches: branches.iter().map(|name| branch(name)).collect(),
            remote_branches: remotes.iter().map(|name| remote(name)).collect(),
            tags: tags.iter().map(|name| tag(name)).collect(),
        };
        let mut remote_names: Vec<String> = references
            .remote_branches
            .iter()
            .map(|branch| branch.remote.clone())
            .collect();
        remote_names.dedup();
        let repository = RepositoryState {
            remotes: remote_names,
            stashes: vec![Stash {
                commit: id("stash"),
                index: 0,
                message: String::from("On main: before the merge"),
                date: Timestamp(0),
            }],
            ..RepositoryState::default()
        };
        Rail {
            repository,
            references,
            lists: RailLists::default(),
            colors: HashMap::new(),
            favorites: Vec::new(),
            recent: Vec::new(),
            typed: String::new(),
            shown: None,
            follows: true,
        }
    }

    fn state(&self) -> ReferencesState<'_> {
        ReferencesState {
            repository: &self.repository,
            references: &self.references,
            lists: &self.lists,
            branch_colors: &self.colors,
            remote_colors: &self.colors,
            favorites: &self.favorites,
            recent: &self.recent,
            submodules: &[],
            typed: &self.typed,
            shown: self.shown.as_ref(),
            follows_timeline: self.follows,
            selected: None,
            sort: SortOrder::Name,
        }
    }

    /// The rows as words: a title in capitals with its count, `+` before an open list kept aside and `-` before a
    /// closed one, two spaces before an entry, `*` after a favorite and `~` after a dimmed one.
    fn rows(&self) -> Vec<String> {
        rail_rows(&self.state(), false).iter().filter_map(row_words).collect()
    }

    /// The entries of one list.
    fn entries_of(&self, list: &RailList) -> Vec<String> {
        rail_rows(&self.state(), false)
            .iter()
            .filter_map(|row| match row {
                RailRow::Entry { list: of, entry, .. } if of == list => Some(entry.name()),
                _ => None,
            })
            .collect()
    }
}

fn row_words(row: &RailRow<'_>) -> Option<String> {
    match row {
        RailRow::KeptTitle => Some(String::from("KEPT ASIDE")),
        RailRow::Heading(head) => Some(format!("{} {}", super::draw::list_name(&head.list), head.all)),
        RailRow::Kept(head) => {
            let mark = if head.is_open { '+' } else { '-' };
            Some(format!("{mark}{} {}", super::draw::list_name(&head.list), head.all))
        }
        RailRow::Entry { entry, marks, .. } => {
            let star = if marks.is_favorite { "*" } else { "" };
            let dim = if marks.is_dimmed { "~" } else { "" };
            Some(format!("  {}{star}{dim}", entry.name()))
        }
        RailRow::InFull { count, .. } => Some(crate::text::rail::show_all_of(*count)),
        RailRow::Gap => None,
    }
}

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| format!("  {name}")).collect()
}

#[test]
fn refs_tree__after_the_start_the_lists_kept_aside_are_closed_and_the_others_open() {
    let rail = Rail::new(&["main", "feature"], &["origin/main"], &["v1.0"]);
    assert_eq!(
        rail.rows(),
        [
            "LOCAL 2",
            "  main",
            "  feature",
            "ORIGIN 1",
            "  origin/main",
            "KEPT ASIDE",
            "-Stashes 1",
            "-Worktrees 0",
            "-Tags 1"
        ]
    );
    let lists = RailLists::default();
    assert!(lists.is_open(&RailList::Local) && lists.is_open(&RailList::Remote(String::from("origin"))));
    assert!(!lists.is_open(&RailList::Tags) && !lists.is_open(&RailList::Stashes));
}

#[test]
fn refs_tree__a_list_of_250_tags_lists_100_and_show_all_250() {
    let tags: Vec<String> = (0..250).map(|index| format!("v{index:03}")).collect();
    let tags: Vec<&str> = tags.iter().map(String::as_str).collect();
    let mut rail = Rail::new(&["main"], &[], &tags);
    let _ = rail.lists.turned.insert(RailList::Tags);
    assert_eq!(rail.entries_of(&RailList::Tags).len(), 100);
    assert_eq!(rail.rows().last().map(String::as_str), Some("Show all 250\u{2026}"));
    let _ = rail.lists.in_full.insert(RailList::Tags);
    assert_eq!(rail.entries_of(&RailList::Tags).len(), 250);
}

#[test]
fn refs_tree__one_ahead_and_two_behind_says_up_1_down_2_and_a_gone_upstream_says_gone() {
    let mut ahead = branch("feature");
    let upstream = Upstream {
        reference: String::from("refs/remotes/origin/feature"),
        remote: String::from("origin"),
        remote_branch: String::from("feature"),
        ahead: 1,
        behind: 2,
        is_gone: false,
    };
    ahead.upstream = Some(upstream.clone());
    assert_eq!(branch_stands(&ahead).as_deref(), Some("\u{2191}1 \u{2193}2"));
    ahead.upstream = Some(Upstream {
        is_gone: true,
        ..upstream
    });
    assert_eq!(branch_stands(&ahead).as_deref(), Some("gone"));
    assert_eq!(branch_stands(&branch("main")), None);
}

#[test]
fn refs_tree__clicking_a_stash_goes_to_the_stash_in_the_timeline_without_narrowing() {
    let rail = Rail::new(&["main"], &[], &[]);
    let state = rail.state();
    let stash = rail.repository.stashes.first().map(Entry::Stash);
    let intents = stash.map(|stash| click_intents(&state, &stash)).unwrap_or_default();
    assert_eq!(intents, [Intent::Graph(GraphIntent::GoTo(id("stash")))]);
}

#[test]
fn refs_tree__the_checked_out_branch_150th_by_name_is_listed_after_the_first_100() {
    let mut branches: Vec<String> = (0..200).map(|index| format!("b{index:03}")).collect();
    let checked_out = String::from("main");
    branches.insert(149, checked_out);
    let branches: Vec<&str> = branches.iter().map(String::as_str).collect();
    let rail = Rail::new(&branches, &[], &[]);
    let local = rail.entries_of(&RailList::Local);
    assert_eq!(local.len(), 101);
    assert_eq!(local.last().map(String::as_str), Some("main"));
    assert!(rail.rows().contains(&String::from("Show all 201\u{2026}")));
}

#[test]
fn refs_tree__a_click_on_a_branch_narrows_to_it_and_selects_its_tip_and_a_second_click_shows_everything() {
    let mut rail = Rail::new(&["main", "feature"], &["origin/main"], &[]);
    let feature = rail.references.branches.get(1).cloned().map(|feature| feature.commit);
    let clicked = click_intents(&rail.state(), &Entry::Branch(&branch("feature")));
    let only = BranchesShown::References(vec![String::from("refs/heads/feature")]);
    assert_eq!(
        clicked,
        [
            Intent::Filter(FilterIntent::Show(Some(only.clone()))),
            Intent::Graph(GraphIntent::GoTo(feature.unwrap_or_else(|| id("feature"))))
        ]
    );
    rail.shown = Some(only);
    let again = click_intents(&rail.state(), &Entry::Branch(&branch("feature")));
    assert_eq!(again, [Intent::Filter(FilterIntent::Show(None))]);
}

#[test]
fn refs_tree__favorites_and_recent_come_first_and_only_when_they_have_something() {
    let mut rail = Rail::new(&["main", "feature", "fix"], &[], &[]);
    rail.recent = vec![String::from("fix")];
    rail.favorites = vec![String::from("refs/heads/feature")];
    let rows = rail.rows();
    assert_eq!(
        rows.get(..4),
        Some(&["FAVORITES 1", "  feature*", "RECENT 1", "  fix"].map(String::from)[..])
    );
    rail.recent.clear();
    rail.favorites.clear();
    assert_eq!(rail.rows().first().map(String::as_str), Some("LOCAL 3"));
}

#[test]
fn refs_filter__with_14_names_there_is_no_box_and_with_15_there_is() {
    let fourteen: Vec<String> = (0..14).map(|index| format!("b{index}")).collect();
    let fourteen: Vec<&str> = fourteen.iter().map(String::as_str).collect();
    let mut rail = Rail::new(&fourteen, &[], &[]);
    rail.repository.stashes.clear();
    assert!(!is_filter_offered(&rail.state()));
    let mut fifteen = Rail::new(&[fourteen.as_slice(), &["one more"]].concat(), &[], &[]);
    fifteen.repository.stashes.clear();
    assert!(is_filter_offered(&fifteen.state()));
    rail.typed = String::from("b1");
    assert!(is_filter_offered(&rail.state()), "something typed keeps the box");
}

#[test]
fn refs_filter__typing_main_lists_main_origin_main_maintenance_release_main_2_then_domain() {
    let names: Vec<String> = ["domain", "release/main-2", "maintenance", "origin/main", "main"]
        .map(String::from)
        .to_vec();
    let ranked: Vec<&str> = rank_names(&names, "main")
        .into_iter()
        .filter_map(|index| names.get(index).map(String::as_str))
        .collect();
    assert_eq!(
        ranked,
        ["main", "origin/main", "maintenance", "release/main-2", "domain"]
    );
    let mut rail = Rail::new(
        &["domain", "release/main-2", "maintenance", "main", "fix"],
        &["origin/main"],
        &[],
    );
    rail.typed = String::from("main");
    let rows = rail.rows();
    assert_eq!(
        rows.get(..7),
        Some(
            &[
                "LOCAL 5",
                "  main",
                "  maintenance",
                "  release/main-2",
                "  domain",
                "ORIGIN 1",
                "  origin/main"
            ]
            .map(String::from)[..]
        )
    );
}

#[test]
fn refs_filter__a_closed_list_with_a_match_is_open_while_the_box_has_text() {
    let mut rail = Rail::new(&["main"], &[], &["v1.0", "v2.0"]);
    rail.typed = String::from("v1");
    assert!(rail.rows().contains(&String::from("+Tags 2")));
    assert_eq!(rail.entries_of(&RailList::Tags), ["v1.0"]);
    rail.typed.clear();
    assert!(rail.rows().contains(&String::from("-Tags 2")));
}

#[test]
fn refs_scope_line__narrowed_to_main_and_feature_the_local_list_has_only_those_two() {
    let mut rail = Rail::new(&["main", "feature", "fix"], &["origin/main"], &[]);
    let shown = BranchesShown::References(vec![
        String::from("refs/heads/main"),
        String::from("refs/heads/feature"),
    ]);
    assert_eq!(model::filter::label(Some(&shown)), "main +1");
    rail.shown = Some(shown);
    assert_eq!(rail.entries_of(&RailList::Local), ["main", "feature"]);
    assert_eq!(
        rail.entries_of(&RailList::Remote(String::from("origin"))),
        [] as [String; 0]
    );
    assert!(
        rail.rows().contains(&String::from("-Stashes 1")),
        "the stashes are always listed"
    );
    assert!(
        !rail.rows().iter().any(|row| row.starts_with("ORIGIN")),
        "a remote with none shown is left out"
    );
}

#[test]
fn refs_scope_line__off_it_lists_every_branch_the_others_dimmed() {
    let mut rail = Rail::new(&["main", "feature", "fix"], &[], &[]);
    rail.shown = Some(BranchesShown::References(vec![String::from("refs/heads/main")]));
    rail.follows = false;
    assert_eq!(rail.rows().get(1..4), Some(&names(&["main", "feature~", "fix~"])[..]));
}

#[test]
fn filter_list_too__narrowed_to_main_on_the_local_section_lists_main_alone() {
    let mut rail = Rail::new(&["main", "feature"], &[], &[]);
    rail.shown = Some(BranchesShown::References(vec![String::from("refs/heads/main")]));
    assert_eq!(rail.entries_of(&RailList::Local), ["main"]);
}

#[test]
fn filter_list_too__off_every_branch_is_listed_all_but_main_dimmed() {
    let mut rail = Rail::new(&["main", "feature"], &[], &[]);
    rail.shown = Some(BranchesShown::References(vec![String::from("refs/heads/main")]));
    rail.follows = false;
    assert_eq!(rail.rows().get(1..3), Some(&names(&["main", "feature~"])[..]));
    let rail_none = Rail::new(&["main", "feature"], &[], &[]);
    assert_eq!(
        rail_none.rows().get(1..3),
        Some(&names(&["main", "feature"])[..]),
        "not narrowed, all are listed"
    );
}

#[test]
fn refs_favorite__add_to_favorites_on_feature_lists_it_under_favorites_with_a_star_under_local() {
    let mut rail = Rail::new(&["main", "feature"], &[], &[]);
    let asked = super::favorite_intents(&Favorites {
        reference: "refs/heads/feature",
        is_favorite: false,
    });
    assert_eq!(
        asked,
        [Intent::Rail(RailIntent::Favorite {
            reference: String::from("refs/heads/feature"),
            is_favorite: true
        })]
    );
    rail.favorites = vec![String::from("refs/heads/feature")];
    assert_eq!(
        rail.rows().get(..5),
        Some(&["FAVORITES 1", "  feature*", "LOCAL 2", "  main", "  feature*"].map(String::from)[..])
    );
}

#[test]
fn refs_favorite__remove_from_favorites_with_none_left_takes_the_list_away() {
    let mut rail = Rail::new(&["main", "feature"], &[], &[]);
    rail.favorites = vec![String::from("refs/heads/feature")];
    let state = rail.state();
    let feature = branch("feature");
    let menu = entry_menu(&state, &Entry::Branch(&feature), "Check out feature");
    let labels: Vec<&str> = menu
        .iter()
        .filter_map(|entry| match entry {
            MenuEntry::Item(item) if item.value == EntryCommand::Favorite => Some(item.label),
            _ => None,
        })
        .collect();
    assert_eq!(labels, ["Remove from favorites"]);
    rail.favorites.clear();
    assert!(!rail.rows().iter().any(|row| row.starts_with("FAVORITES")));
}

#[test]
fn refs_favorite__a_favorite_deleted_with_branch_d_is_no_longer_listed() {
    let mut rail = Rail::new(&["main"], &[], &[]);
    rail.favorites = vec![String::from("refs/heads/feature")];
    assert!(!rail.rows().iter().any(|row| row.starts_with("FAVORITES")));
}

/// The entries of the menu of a reference about the branches shown.
fn filter_entries_of(rail: &Rail, entry: &Entry<'_>) -> Vec<FilterEntry> {
    let state = rail.state();
    entry_menu(&state, entry, "")
        .iter()
        .filter_map(|item| match item {
            MenuEntry::Item(item) => match item.value {
                EntryCommand::Filter(filter) => Some(filter),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[test]
fn filter_show_only__on_the_reference_already_shown_alone_the_menu_offers_show_all_branches_instead() {
    let mut rail = Rail::new(&["main", "feature"], &[], &["v1.0"]);
    let v1 = tag("v1.0");
    assert_eq!(filter_entries_of(&rail, &Entry::Tag(&v1)), [FilterEntry::ShowOnly]);
    rail.shown = Some(BranchesShown::References(vec![String::from("refs/tags/v1.0")]));
    assert_eq!(filter_entries_of(&rail, &Entry::Tag(&v1)), [FilterEntry::ShowAll]);
}

#[test]
fn filter_show_too__with_the_timeline_not_narrowed_the_entry_is_not_offered() {
    let mut rail = Rail::new(&["main", "feature"], &[], &[]);
    let feature = branch("feature");
    assert!(!filter_entries_of(&rail, &Entry::Branch(&feature)).contains(&FilterEntry::ShowToo));
    rail.shown = Some(BranchesShown::References(vec![String::from("refs/heads/main")]));
    assert_eq!(
        filter_entries_of(&rail, &Entry::Branch(&feature)),
        [FilterEntry::ShowOnly, FilterEntry::ShowToo, FilterEntry::ShowAll]
    );
}

#[test]
fn filter_hide__on_the_only_reference_shown_the_entry_is_not_offered() {
    let mut rail = Rail::new(&["main", "feature"], &[], &[]);
    rail.shown = Some(BranchesShown::References(vec![String::from("refs/heads/main")]));
    let main = branch("main");
    assert!(!filter_entries_of(&rail, &Entry::Branch(&main)).contains(&FilterEntry::Hide));
    rail.shown = Some(BranchesShown::References(vec![
        String::from("refs/heads/main"),
        String::from("refs/heads/feature"),
    ]));
    assert!(filter_entries_of(&rail, &Entry::Branch(&main)).contains(&FilterEntry::Hide));
}
