//! Opening and closing tabs from the window (`repo.choose-tabs`, `repo.open-tab`, `repo.close-tab`,
//! `repo.open-code-workspace`): what the picker of the tabs, **+**, the **×** of a tab, a middle click, the menu of a
//! tab and the Welcome page ask, over three clones made for each test.

use app::persisted::same_folder;
use app::store::{Message, State};
use platform::accesskit::Role;
use render::ThemeName;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey, Point, PointerButton, PointerButtonEvent};
use views::Intent;
use views::text::repo as words;

use super::inspector::ask;
use super::pointer::{bounds, middle, moved};
use super::support::{self, Window};
use super::tabs::{WIDE, clone_three, three_clones};

/// A window asked which of `api`, `web` and `docs` to open in tabs, and what keeps their folder alive.
fn asking() -> (impl Sized, [std::path::PathBuf; 3], State) {
    let (kept, clones) = clone_three();
    let mut state = super::window();
    let _ = support::run_asking(
        &mut state,
        Message::Started {
            folders: clones.to_vec(),
        },
    );
    (kept, clones, state)
}

/// A key pressed alone.
fn press(key: NamedKey) -> InputEvent {
    InputEvent::KeyPressed(KeyEvent::press(Key::Named(key), Modifiers::NONE))
}

/// A button of the pointer pressed and let go at a point, the pointer moved there first.
fn button_at(window: &mut Window, state: &State, point: Point, button: PointerButton) -> Vec<Intent> {
    let mut intents = window.draw(state, &[moved(point)]).0;
    let press = PointerButtonEvent {
        button,
        position: point,
        modifiers: Modifiers::NONE,
        click_count: 1,
    };
    intents.extend(
        window
            .draw(
                state,
                &[InputEvent::PointerPressed(press), InputEvent::PointerReleased(press)],
            )
            .0,
    );
    intents
}

/// How many buttons are named so.
fn count(tree: &platform::accesskit::TreeUpdate, name: &str) -> usize {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some(name))
        .count()
}

#[test]
fn repo_choose_tabs__the_picker_has_every_repository_checked_and_enter_opens_them_all() {
    let (_kept, _clones, state) = asking();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = window.draw(&state, &[]);
    let (intents, _) = window.draw(&state, &[press(NamedKey::Enter)]);
    assert_eq!(intents, [Intent::TabsChosen(Some(vec![0, 1, 2]))]);
}

#[test]
fn repo_open_tab__plus_offers_clone_and_create_a_repository_after_open_folder() {
    let (_kept, _clones, mut state) = asking();
    ask(&mut state, Intent::TabsChosen(Some(vec![0])));
    for (entry, feature) in [(words::CLONE, "repo.clone"), (words::CREATE, "repo.init")] {
        let mut window = Window::new(ThemeName::Dark, WIDE);
        let _ = window.click(&state, Role::Button, words::OPEN_ANOTHER);
        let intents = window.click(&state, Role::MenuItem, entry);
        let run =
            views::feature::FeatureIntent::Run(model::contract::FeatureId(feature), model::contract::Target::Head);
        assert_eq!(intents, [Intent::Feature(run)], "{entry}");
    }
}

#[test]
fn repo_choose_tabs__escape_in_the_picker_opens_nothing() {
    let (_kept, _clones, state) = asking();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = window.draw(&state, &[]);
    let (intents, _) = window.draw(&state, &[press(NamedKey::Escape)]);
    assert_eq!(intents, [Intent::TabsChosen(None)]);
}

#[test]
fn repo_open_tab__plus_lists_web_and_docs_when_api_alone_is_open_and_web_chosen_asks_for_it() {
    let (_kept, clones, mut state) = asking();
    ask(&mut state, Intent::TabsChosen(Some(vec![0])));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = window.click(&state, Role::Button, words::OPEN_ANOTHER);
    let (_, tree) = window.draw(&state, &[]);
    let entries: Vec<&str> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::MenuItem)
        .filter_map(|(_, node)| node.label())
        .collect();
    assert_eq!(
        entries,
        [
            "web",
            "docs",
            views::text::header::OPEN_FOLDER,
            words::CLONE,
            words::CREATE,
            words::OPEN_WORKSPACE_FILE
        ]
    );
    let intents = window.click(&state, Role::MenuItem, "web");
    let [Intent::OpenTab(folder)] = intents.as_slice() else {
        panic!("web is asked for: {intents:?}");
    };
    assert!(same_folder(folder, &clones[1]));
}

#[test]
fn repo_close_tab__the_x_of_the_tab_shown_a_middle_click_and_the_menu_of_a_tab_ask_to_close_it() {
    let (_kept, _clones, mut state) = three_clones();
    ask(&mut state, Intent::SwitchTab(1));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[moved(Point::new(500.0, 570.0))]);
    assert_eq!(count(&tree, words::CLOSE_TAB), 1, "only the tab shown has its ×");
    let docs = middle(bounds(&tree, Role::Button, "docs"));
    let (_, tree) = window.draw(&state, &[moved(docs)]);
    assert_eq!(
        count(&tree, words::CLOSE_TAB),
        2,
        "the tab under the pointer has its × too"
    );
    assert_eq!(
        window.click(&state, Role::Button, words::CLOSE_TAB),
        [Intent::CloseTab(1)]
    );
    assert_eq!(
        button_at(&mut window, &state, docs, PointerButton::Middle),
        [Intent::CloseTab(2)]
    );
    let api = middle(bounds(&tree, Role::Button, "api"));
    let _ = button_at(&mut window, &state, api, PointerButton::Secondary);
    assert_eq!(
        window.click(&state, Role::MenuItem, words::CLOSE_OTHER_TABS),
        [Intent::CloseOtherTabs(0)]
    );
}

#[test]
fn repo_open_code_workspace__the_welcome_page_offers_open_workspace_file() {
    let state = super::window();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    assert_eq!(
        window.click(&state, Role::Button, words::OPEN_WORKSPACE_FILE),
        [Intent::OpenWorkspaceFile]
    );
}
