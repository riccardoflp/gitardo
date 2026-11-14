//! The gallery with each overlay open — a menu with a submenu, a dialog, the dialog of Pull, a confirmation, a picker,
//! the find bar, a drag and the Activity drawer — in the dark and light themes at 100 % and in the dark theme at 200 %, compared with their reference images:
//! `GITARDO_GOLDEN_RECORD=1 cargo test -p ui --test overlays` records them again.

#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]

mod support;

use render::ThemeName;
use support::Window;
use ui::gallery::{self, Scene};

/// The size of the window of the gallery, in logical pixels.
const SIZE: (u32, u32) = (1024, 720);

/// Draws the gallery with a scene open and compares it with its snapshot.
fn check(scene: Scene, theme: ThemeName, scale: u8, name: &str) {
    let mut window = Window::new(theme, SIZE, scale);
    let state = gallery::prepare_scene(&mut window.memory, &window.theme, scene);
    let (intents, _) = window.frame(&[], |ui| gallery::view(&state, ui));
    assert!(
        intents.is_empty(),
        "a frame with no input asks for nothing: {intents:?}"
    );
    window.assert_snapshot(name);
}

/// The three snapshots of a scene.
fn check_all(scene: Scene, name: &str) {
    check(scene, ThemeName::Dark, 1, &format!("{name}-dark"));
    check(scene, ThemeName::Light, 1, &format!("{name}-light"));
    check(scene, ThemeName::Dark, 2, &format!("{name}-dark-2x"));
}

#[test]
fn a_context_menu_with_a_submenu_open() {
    check_all(Scene::Menu, "menu");
}

#[test]
fn a_dialog() {
    check_all(Scene::Dialog, "dialog");
}

#[test]
fn the_dialog_of_pull_with_its_choices() {
    check_all(Scene::Pull, "pull");
}

#[test]
fn the_activity_drawer() {
    check_all(Scene::Activity, "activity");
}

#[test]
fn a_confirmation_of_something_dangerous() {
    check_all(Scene::Confirmation, "confirmation");
}

#[test]
fn a_picker_with_a_filter_typed() {
    check_all(Scene::Picker, "picker");
}

#[test]
fn the_find_bar_with_its_matches() {
    check_all(Scene::Find, "find");
}

#[test]
fn a_drag_in_progress() {
    check_all(Scene::Drag, "drag");
}
