//! The island of the tabs in a narrow window (`repo.tabs`): the tabs narrowed, those that still do not fit in **… N**,
//! the island as one button below 480 logical pixels, and its snapshots, over clones made for each test.

use std::path::PathBuf;

use app::store::{Message, State, ViewData};
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use views::Intent;

use super::inspector::ask;
use super::support::{self, NOW, Window, find};

/// Eight repositories whose names are longer than a tab narrowed.
const EIGHT: [&str; 8] = [
    "accounts-service",
    "billing-service",
    "catalog-service",
    "delivery-service",
    "identity-service",
    "messaging-service",
    "payments-service",
    "reporting-service",
];

/// A repository with a little history, cloned under each name in a folder of their own, and a window showing them
/// all, the first shown; the folder is kept alive by the first value.
fn clones(names: &[&str]) -> ((Repository, Repository), Vec<PathBuf>, State) {
    let mut origin = Repository::new();
    let _ = origin.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let holder = Repository::new();
    let folders: Vec<PathBuf> = names
        .iter()
        .map(|name| holder.path().join("clones").join(name))
        .collect();
    for folder in &folders {
        let _ = origin.git(&["clone", "--quiet", ".", folder.to_str().unwrap()]);
        let folder = folder.to_str().unwrap();
        let _ = origin.git(&["-C", folder, "config", "user.name", "Test Author"]);
        let _ = origin.git(&["-C", folder, "config", "user.email", "author@example.com"]);
    }
    let mut state = super::window();
    let _ = support::run(
        &mut state,
        Message::Started {
            folders: folders.clone(),
        },
    );
    ((origin, holder), folders, state)
}

/// The buttons of the tree whose names start with `…`: **… N**.
fn more_buttons(tree: &TreeUpdate) -> Vec<String> {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button)
        .filter_map(|(_, node)| node.label())
        .filter(|label| label.starts_with('\u{2026}'))
        .map(str::to_owned)
        .collect()
}

/// The names of the entries of the menu open.
fn menu_items(tree: &TreeUpdate) -> Vec<String> {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::MenuItem)
        .filter_map(|(_, node)| node.label())
        .map(str::to_owned)
        .collect()
}

/// The names of the tabs drawn as tabs.
fn drawn(tree: &TreeUpdate) -> Vec<&'static str> {
    EIGHT
        .into_iter()
        .filter(|name| {
            tree.nodes
                .iter()
                .any(|(_, node)| node.role() == Role::Button && node.label() == Some(*name))
        })
        .collect()
}

#[test]
fn repo_tabs__with_eight_tabs_in_a_window_900_wide_the_others_are_in_more_and_never_the_tab_shown() {
    let (_kept, _folders, mut state) = clones(&EIGHT);
    ask(&mut state, Intent::SwitchTab(7));
    let mut window = Window::new(ThemeName::Dark, (900, 580));
    let (_, tree) = window.draw(&state, &[]);
    let shown = drawn(&tree);
    assert!(
        shown.contains(&"reporting-service"),
        "the tab shown is drawn: {shown:?}"
    );
    let more = more_buttons(&tree);
    let listed = EIGHT.len() - shown.len();
    assert!(listed > 0, "eight tabs do not fit");
    assert_eq!(more, [format!("\u{2026} {listed}")]);
    let _ = window.click(&state, Role::Button, &more[0]);
    let (_, tree) = window.draw(&state, &[]);
    let items = menu_items(&tree);
    assert_eq!(items.len(), listed);
    assert!(!items.iter().any(|item| item == "reporting-service"));
    let first_listed = items[0].clone();
    let at = EIGHT.iter().position(|name| *name == first_listed).unwrap();
    assert_eq!(
        window.click(&state, Role::MenuItem, &first_listed),
        [Intent::SwitchTab(at)]
    );
}

#[test]
fn repo_tabs__below_480_logical_pixels_the_island_is_one_button_whose_list_has_every_tab_and_open_a_repository() {
    let (_kept, _folders, mut state) = clones(&["api", "web", "docs"]);
    ask(&mut state, Intent::SwitchTab(1));
    let mut window = Window::new(ThemeName::Dark, (460, 580));
    let (_, tree) = window.draw(&state, &[]);
    let _ = find(&tree, Role::Button, "web");
    assert!(
        !tree
            .nodes
            .iter()
            .any(|(_, node)| node.role() == Role::Button && node.label() == Some("api")),
        "no other tab is drawn"
    );
    let _ = window.click(&state, Role::Button, "web");
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(
        menu_items(&tree),
        ["api", "web", "docs", views::text::repo::OPEN_A_REPOSITORY]
    );
    assert_eq!(
        window.click(&state, Role::MenuItem, views::text::repo::OPEN_A_REPOSITORY),
        [Intent::PickTabToOpen]
    );
}

#[test]
fn repo_tabs__the_snapshots_of_the_island_wide_narrow_and_as_one_button_match_in_every_theme_at_both_scales() {
    let (_three, _, mut three) = clones(&["api", "web", "docs"]);
    ask(&mut three, Intent::SwitchTab(1));
    let (_eight, _, mut eight) = clones(&EIGHT);
    ask(&mut eight, Intent::SwitchTab(6));
    let themes = [
        (ThemeName::Dark, "dark"),
        (ThemeName::Light, "light"),
        (ThemeName::HighContrastDark, "high-contrast-dark"),
        (ThemeName::HighContrastLight, "high-contrast-light"),
    ];
    let layouts = [
        (&three, 900, "wide"),
        (&eight, 700, "narrow"),
        (&three, 420, "one-button"),
    ];
    for (theme, theme_name) in themes {
        for scale in [1, 2] {
            for (state, width, layout) in layouts {
                let mut window = Window::scaled(theme, (width, 48), scale);
                let data = ViewData::new(state, Vec::new(), NOW);
                let window_state = data.window_state(state);
                let tabs = window_state.tabs.unwrap();
                let _ = window.frame(&[], |ui| views::repo::tabs_view(&tabs, ui));
                let suffix = if scale == 2 { "-2x" } else { "" };
                window.assert_snapshot(&format!("tabs-{layout}-{theme_name}{suffix}"));
            }
        }
    }
}
