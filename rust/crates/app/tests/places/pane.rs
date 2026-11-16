//! The branch rail as one place, drawn from the store (`branch.pane`): its top with the branch checked out and its
//! buttons, the branches the timeline leaves out dimmed, a click on a branch, and the lists open when the window opens.

use app::store::{Message, State};
use model::contract::{FeatureId, Target};
use model::filter::BranchesShown;
use platform::accesskit::{Role, TreeUpdate};
use render::{Rect, ThemeName};
use views::feature::FeatureIntent;
use views::graph::Selected;
use views::{FilterIntent, Intent, RailList};

use super::pointer::{apply, bounds, middle, moved, rect};
use super::support::{Window, has, run};
use super::{repository_with_history, showing};

/// The size of the window of these tests.
const WIDE: (u32, u32) = (1000, 520);

/// The rectangle of a button of the branch rail by its name, below its top, when the rail has one.
fn rail_entry(tree: &TreeUpdate, name: &str) -> Option<Rect> {
    let rail = bounds(tree, Role::Navigation, "Branches");
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some(name))
        .filter_map(|(_, node)| node.bounds().map(rect))
        .find(|found| found.x >= rail.x && found.right() <= rail.right() + 0.5 && found.y >= rail.y)
}

/// Whether a button of the branch rail has a name, below its top.
fn listed(tree: &TreeUpdate, name: &str) -> bool {
    rail_entry(tree, name).is_some()
}

/// The commit a branch of the repository shown is at.
fn tip_of(state: &State, name: &str) -> model::CommitId {
    let read = &state.shown_read().unwrap().state;
    read.branches
        .iter()
        .find(|branch| branch.name == name)
        .unwrap()
        .commit
        .clone()
}

/// The timeline narrowed to `main`.
fn narrowed_to_main(state: &mut State) {
    let shown = BranchesShown::References(vec![String::from("refs/heads/main")]);
    let _effects = run(state, Message::Intent(Intent::Filter(FilterIntent::Show(Some(shown)))));
}

#[test]
fn branch_pane__the_top_names_the_branch_checked_out_and_create_branch_there_asks_for_a_branch_at_head() {
    let (repository, _remote) = repository_with_history();
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let rail = bounds(&tree, Role::Navigation, "Branches");
    let create = bounds(&tree, Role::Button, "Create branch");
    assert!(create.bottom() <= rail.y + 0.5, "the button is above the lists");
    let is_named_on_top = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.value() == Some("main") || node.label() == Some("main"))
        .filter_map(|(_, node)| node.bounds().map(rect))
        .any(|found| found.x >= rail.x && found.right() <= rail.right() + 0.5 && found.bottom() <= create.y + 0.5);
    assert!(is_named_on_top, "the top of the rail says main");
    let intents = window.click(&state, Role::Button, "Create branch");
    assert_eq!(
        intents,
        [Intent::Feature(FeatureIntent::Run(
            FeatureId("branch.create"),
            Target::Head
        ))]
    );
    let compared = window.click(&state, Role::Button, "Compare");
    assert_eq!(
        compared,
        [Intent::Feature(FeatureIntent::Run(
            FeatureId("compare.choose"),
            Target::Head
        ))]
    );
}

#[test]
fn branch_pane__with_the_timeline_narrowed_to_main_the_rail_lists_every_local_branch_the_others_dimmed() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    narrowed_to_main(&mut state);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(listed(&tree, "main"));
    assert!(listed(&tree, "feature"));
    let main = rail_entry(&tree, "main").unwrap();
    let (_, tree) = window.draw(&state, &[moved(middle(main))]);
    assert!(!has(&tree, Role::Button, "Show this history too"), "main is not dimmed");
    let feature = rail_entry(&tree, "feature").unwrap();
    let (_, tree) = window.draw(&state, &[moved(middle(feature))]);
    assert!(
        has(&tree, Role::Button, "Show this history too"),
        "feature is dimmed, with the eye that adds it"
    );
}

#[test]
fn branch_pane__a_click_on_feature_narrows_to_it_and_selects_its_tip_and_a_second_click_brings_everything_back() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let everything = state.timeline.rows.len();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, "feature");
    apply(&mut state, intents);
    let only = BranchesShown::References(vec![String::from("refs/heads/feature")]);
    assert_eq!(state.timeline.shown, Some(only));
    assert_eq!(
        state.timeline.selected,
        Some(Selected::Commit(tip_of(&state, "feature")))
    );
    assert!(state.pages.open.is_empty(), "no page is opened");
    let again = window.click(&state, Role::Button, "feature");
    assert!(again.contains(&Intent::Filter(FilterIntent::Show(None))), "{again:?}");
    apply(&mut state, again);
    assert_eq!(state.timeline.shown, None);
    assert_eq!(state.timeline.rows.len(), everything);
}

#[test]
fn branch_pane__when_the_window_opens_the_lists_kept_aside_are_closed_and_the_others_open() {
    let (repository, _remote) = repository_with_history();
    let state = showing(repository.path().to_path_buf());
    for kept_aside in [
        RailList::Stashes,
        RailList::Worktrees,
        RailList::Tags,
        RailList::Submodules,
    ] {
        assert!(!state.rail.lists.is_open(&kept_aside), "{kept_aside:?} is closed");
    }
    assert!(state.rail.lists.is_open(&RailList::Local));
    assert!(state.rail.lists.is_open(&RailList::Remote(String::from("origin"))));
}
