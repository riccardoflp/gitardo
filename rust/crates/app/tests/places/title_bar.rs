//! The header bar as the title bar of the window (`window.title-bar`, `window.caption-buttons`): the three buttons at
//! its right end, what they ask, what the screen reader names them, and the places of the header bar that are its
//! caption, over clones made for each test.

use app::store::{Effect, Message, State};
use app::window::frame::frame_part;
use platform::FramePart;
use platform::accesskit::Role;
use render::{Point, ThemeName};
use testkit::Repository;
use ui::{InputEvent, PointerMove};
use views::Intent;
use views::window::{CaptionState, FrameState, Hit, TitleBar, WindowIntent, WindowSize};

use super::side_column::{HEADER, header_button, sync};
use super::support::{self, Window};
use super::tabs::{WIDE, three_clones};

/// The three clones shown in a window that draws its title bar, maximized or not.
fn drawn(is_maximized: bool) -> ((Repository, Repository), State) {
    let (kept, _clones, mut state) = three_clones();
    let caption = CaptionState {
        title_bar: TitleBar::Drawn,
        is_maximized,
    };
    let _effects = support::run(&mut state, Message::Caption(caption));
    (kept, state)
}

/// The width of the window of the tests, in logical pixels.
#[allow(clippy::cast_precision_loss, reason = "the window is a few hundred pixels wide")]
fn width() -> f32 {
    WIDE.0 as f32
}

#[test]
fn window_caption_buttons__on_windows_the_header_bar_ends_with_minimize_maximize_and_close_after_more() {
    let (_kept, state) = drawn(false);
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    let more = header_button(&tree, "More");
    let minimize = header_button(&tree, "Minimize");
    let maximize = header_button(&tree, "Maximize");
    let close = header_button(&tree, "Close");
    assert!(more.right() <= minimize.x, "{more:?} before {minimize:?}");
    assert!(minimize.right() <= maximize.x && maximize.right() <= close.x);
    assert!(
        (close.right() - width()).abs() < 0.5,
        "Close reaches the right edge: {close:?}"
    );
    assert!(
        close.y.abs() < 0.5 && (close.height - HEADER).abs() < 0.5,
        "as high as the header bar: {close:?}"
    );
    assert!(
        (close.width - 46.0).abs() < 0.5,
        "as wide as Windows draws it: {close:?}"
    );
}

#[test]
fn window_caption_buttons__minimize_minimizes_and_maximize_maximizes_and_becomes_restore_which_restores() {
    let (_kept, mut state) = drawn(false);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, "Minimize");
    assert_eq!(intents, vec![Intent::Window(WindowIntent::Minimize)]);
    let intents = window.click(&state, Role::Button, "Maximize");
    assert_eq!(intents, vec![Intent::Window(WindowIntent::Maximize)]);
    let effects = support::run(&mut state, Message::Intent(Intent::Window(WindowIntent::Maximize)));
    assert!(effects.contains(&Effect::Window(WindowIntent::Maximize)), "{effects:?}");
    let (_kept, maximized) = drawn(true);
    let intents = window.click(&maximized, Role::Button, "Restore");
    assert_eq!(intents, vec![Intent::Window(WindowIntent::Restore)]);
}

#[test]
fn window_caption_buttons__close_closes_the_window_as_the_close_request_of_the_system() {
    let (_kept, mut state) = drawn(false);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, "Close");
    assert_eq!(intents, vec![Intent::Window(WindowIntent::Close)]);
    let effects = support::run(&mut state, Message::Intent(Intent::Window(WindowIntent::Close)));
    assert!(effects.contains(&Effect::Window(WindowIntent::Close)), "{effects:?}");
    // The window answers it with the close request of the system, which keeps its size and quits.
    let effects = support::run(
        &mut state,
        Message::CloseRequested {
            window_size: Some((1000.0, 580.0)),
        },
    );
    assert!(effects.contains(&Effect::Quit), "{effects:?}");
    assert_eq!(state.persisted.window_size, Some((1000.0, 580.0)));
}

#[test]
fn window_caption_buttons__the_pointer_on_close_draws_it_red() {
    let (_kept, state) = drawn(false);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let close = header_button(&tree, "Close");
    let corner = Point::new(close.x + 4.0, close.y + 4.0);
    let moved = InputEvent::PointerMoved(PointerMove {
        position: corner,
        modifiers: ui::Modifiers::NONE,
    });
    let _ = window.draw(&state, &[moved]);
    let _ = window.draw(&state, &[]);
    let pixels = window.frame.to_rgba();
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "inside the window"
    )]
    let at = (corner.y as usize * WIDE.0 as usize + corner.x as usize) * 4;
    assert_eq!(pixels[at..at + 3], [0xC4, 0x2B, 0x1C]);
}

#[test]
fn window_caption_buttons__the_screen_reader_names_minimize_maximize_and_close_and_restore_while_maximized() {
    let (_kept, state) = drawn(false);
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    for name in ["Minimize", "Maximize", "Close"] {
        assert!(support::has(&tree, Role::Button, name), "{name}");
    }
    assert!(!support::has(&tree, Role::Button, "Restore"));
    let (_kept, maximized) = drawn(true);
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&maximized, &[]);
    assert!(support::has(&tree, Role::Button, "Restore"));
    assert!(!support::has(&tree, Role::Button, "Maximize"));
}

#[test]
fn window_title_bar__as_drawn_the_free_header_bar_is_the_caption_and_search_sync_and_the_repository_are_not() {
    let (_kept, state) = drawn(false);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let frame = FrameState {
        caption: state.caption,
        caption_height: HEADER,
        controls: window.pointer_targets(ui::Rect::new(0.0, 0.0, width(), HEADER)),
    };
    let size = WindowSize {
        width: width(),
        height: 580.0,
    };
    let at = |point: Point| frame_part(views::window::hit(point, size, &frame));
    let repository = header_button(&tree, "api");
    let search = header_button(&tree, "Search");
    let (_, sync) = sync(&tree);
    for control in [repository, search, sync] {
        assert_eq!(at(control.center()), FramePart::Client, "{control:?}");
    }
    let free = Point::new(f32::midpoint(repository.right(), search.x), HEADER * 0.5);
    assert_eq!(at(free), FramePart::Caption);
    assert_eq!(views::window::hit(free, size, &frame), Hit::Caption);
    assert_eq!(at(Point::new(width() - 10.0, 20.0)), FramePart::Close);
}

#[test]
fn window_caption_buttons__the_header_bar_with_the_buttons_in_each_theme() {
    let (_kept, state) = drawn(false);
    for (theme, name) in [
        (ThemeName::Dark, "title-bar-dark"),
        (ThemeName::Light, "title-bar-light"),
        (ThemeName::HighContrastDark, "title-bar-high-contrast-dark"),
        (ThemeName::HighContrastLight, "title-bar-high-contrast-light"),
    ] {
        let mut window = Window::new(theme, (1000, 160));
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(name);
    }
}
