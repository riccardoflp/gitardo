//! The ways into the review of the conflicts besides Quick search: **Review** in the question of
//! `conflicts.resolve-automatically`, the button of the banner, and the wand of a file on the Changes page.

use app::settings::Settings;
use app::store::{Message, State};
use model::contract::{Answer, Answers, FeatureId, Target};
use platform::accesskit::Role;
use render::ThemeName;
use views::Intent;
use views::changes::ChangesIntent;
use views::feature::FeatureIntent;
use views::page::Page;

use super::review::{NEXT_TO_EACH_OTHER, SAME_LINE, cards, file, merged};
use super::support::{OperationsDriver, Window, run};
use super::timeline::showing_with;

/// The settings with `conflicts.autoResolve` on.
fn auto_resolve() -> Settings {
    let mut settings = Settings::default();
    settings.features.conflicts_auto_resolve = true;
    settings
}

/// Gives each intent to the store, with the jobs it starts.
fn reduce_all(state: &mut State, intents: Vec<Intent>) {
    for intent in intents {
        let _effects = run(state, Message::Intent(intent));
    }
}

/// The files the review page is about, when not every file in conflict.
fn only(state: &State) -> Option<Vec<String>> {
    state.reviews.page.as_ref().and_then(|page| page.only.clone())
}

#[test]
fn conflicts_resolve_automatically__review_in_the_window_opens_the_review_page_and_writes_nothing() {
    let repository = merged(&[file("one.txt", NEXT_TO_EACH_OTHER), file("two.txt", SAME_LINE)]);
    let before = std::fs::read(repository.path().join("one.txt")).unwrap();
    let mut state = showing_with(repository.path().to_path_buf(), auto_resolve());
    let driver = OperationsDriver::new();
    let answers = Answers::new().with(features::conflicts::RESOLVE, Answer::Choice(String::from("review")));
    let resolve = FeatureIntent::RunWith(FeatureId("conflicts.resolve-automatically"), Target::Head, answers);
    driver.intent(&mut state, Intent::Feature(resolve));
    while !matches!(driver.next(&mut state), Message::FeatureEnded { .. }) {}
    assert_eq!(state.page_shown(), Some(Page::Review));
    assert_eq!(
        only(&state),
        Some(vec![String::from("one.txt"), String::from("two.txt")])
    );
    assert_eq!(cards(&state).len(), 2);
    assert_eq!(state.log.entries().count(), 0, "nothing ran");
    assert_eq!(std::fs::read(repository.path().join("one.txt")).unwrap(), before);
}

#[test]
fn conflicts_banner__resolve_automatically_opens_the_review_of_every_file() {
    let repository = merged(&[file("one.txt", NEXT_TO_EACH_OTHER)]);
    let mut state = showing_with(repository.path().to_path_buf(), auto_resolve());
    let mut window = Window::new(ThemeName::Dark, (1200, 560));
    let intents = window.click(&state, Role::Button, "Resolve it automatically");
    reduce_all(&mut state, intents);
    assert_eq!(state.page_shown(), Some(Page::Review));
    assert_eq!(only(&state), None);
    assert_eq!(cards(&state).len(), 1);
}

#[test]
fn review_page__the_wand_of_a_file_on_the_changes_page_opens_the_review_of_that_file_alone() {
    let repository = merged(&[file("one.txt", NEXT_TO_EACH_OTHER), file("two.txt", SAME_LINE)]);
    let mut state = showing_with(repository.path().to_path_buf(), auto_resolve());
    let _effects = run(&mut state, Message::Intent(Intent::Changes(ChangesIntent::Open)));
    let mut window = Window::new(ThemeName::Dark, (1200, 560));
    for path in ["two.txt", "one.txt"] {
        let items = super::changes_menu::menu_of(&mut window, &state, path);
        let has_wand = items
            .iter()
            .any(|item| item == views::text::changes::RESOLVE_AUTOMATICALLY);
        assert!(has_wand, "the menu of {path}: {items:?}");
    }
    let _ = super::changes_menu::menu_of(&mut window, &state, "one.txt");
    let intents = window.click(&state, Role::MenuItem, views::text::changes::RESOLVE_AUTOMATICALLY);
    reduce_all(&mut state, intents);
    assert_eq!(state.page_shown(), Some(Page::Review));
    assert_eq!(only(&state).map(|files| files.len()), Some(1));
    let paths: Vec<String> = cards(&state).into_iter().map(|card| card.key.path).collect();
    assert_eq!(paths.len(), 1);
}

#[test]
fn review_page__without_conflicts_auto_resolve_the_changes_page_has_no_wand() {
    let repository = merged(&[file("one.txt", NEXT_TO_EACH_OTHER)]);
    let mut state = showing_with(repository.path().to_path_buf(), Settings::default());
    let _effects = run(&mut state, Message::Intent(Intent::Changes(ChangesIntent::Open)));
    let items = super::changes_menu::menu_of(&mut Window::new(ThemeName::Dark, (1200, 560)), &state, "one.txt");
    assert!(
        !items
            .iter()
            .any(|item| item == views::text::changes::RESOLVE_AUTOMATICALLY),
        "{items:?}"
    );
}
