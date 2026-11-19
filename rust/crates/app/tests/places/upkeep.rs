//! The upkeep of a repository from the menu of its row in **Repositories** (`upkeep.clean-up`,
//! `upkeep.start-background`, `upkeep.stop-background`). Nothing is run: `git maintenance start` would schedule git in
//! the system's scheduler, so the tests look at the menu and at what it asks for.

use model::contract::{FeatureId, Target};
use platform::accesskit::Role;
use render::ThemeName;
use views::Intent;
use views::feature::FeatureIntent;
use views::text::repo as words;

use super::changes_menu::menu_of;
use super::support::Window;

#[test]
fn upkeep_clean_up__the_menu_of_the_row_shown_has_clean_up_and_look_after_it_in_the_background() {
    let repositories = super::group_tabs::made(false);
    let state = super::group_tabs::window(&repositories);
    let mut window = Window::new(ThemeName::Dark, super::tabs::WIDE);
    let items = menu_of(&mut window, &state, "api");
    assert!(items.iter().any(|item| item == words::CLEAN_UP), "{items:?}");
    assert!(items.iter().any(|item| item == words::LOOK_AFTER), "{items:?}");
    let intents = window.click(&state, Role::MenuItem, words::CLEAN_UP);
    let clean_up = Intent::Feature(FeatureIntent::Run(FeatureId("upkeep.clean-up"), Target::Head));
    assert!(intents.contains(&clean_up), "{intents:?}");
    let items = menu_of(&mut window, &state, "web");
    assert!(
        !items.iter().any(|item| item == words::CLEAN_UP),
        "only the row shown: {items:?}"
    );
}
