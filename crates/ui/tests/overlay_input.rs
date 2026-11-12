//! The overlays of the gallery driven over several frames with synthetic input, as a person would: menus with the
//! pointer and the keys, a dialog, a confirmation, the pickers, Quick search, the Activity drawer, and dragging the
//! pill of a branch onto the timeline.

#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]

mod support;

use render::ThemeName;
use support::{Window, click, key, move_to, press, release, shortcut, typed};
use ui::gallery::{self, GalleryState, Overlay};
use ui::{FrameOutput, InputEvent, Modifiers, NamedKey, PointerButton, PointerButtonEvent, Ui};

/// Where a commit of the timeline is in the gallery, with "Speed up the reading of large histories".
const ROW: (f32, f32) = (600.0, 394.0);
/// The commit of the timeline at [`ROW`].
const ROW_NUMBER: usize = 4;
/// Where the pill of the repository is, in the header.
const REPOSITORY: (f32, f32) = (66.0, 28.0);

/// The gallery in a window of its size, driven frame by frame, with what it asked for applied to its state.
struct Gallery {
    /// The window.
    window: Window,
    /// The state of the gallery.
    state: GalleryState,
}

impl Gallery {
    /// The gallery drawn once in its fixed state.
    fn new() -> Gallery {
        let mut window = Window::new(ThemeName::Dark, (1280, 800), 1);
        gallery::prepare(&mut window.memory, &window.theme);
        let mut gallery = Gallery {
            window,
            state: GalleryState::default(),
        };
        let _ = gallery.step(&[]);
        gallery
    }

    /// Draws a frame with some input, applies what it asked for, and moves the time on.
    fn step(&mut self, events: &[InputEvent]) -> FrameOutput {
        self.step_and(events, |_| ()).1
    }

    /// Draws a frame with some input, then asks the toolkit something in the scope of the gallery.
    fn step_and<R>(&mut self, events: &[InputEvent], ask: impl FnOnce(&mut Ui<'_>) -> R) -> (R, FrameOutput) {
        let state = self.state.clone();
        let ((intents, answer), output) = self.window.frame(events, |ui| {
            let intents = gallery::view(&state, ui);
            let answer = ui.scope("gallery", ask);
            (intents, answer)
        });
        for intent in &intents {
            self.state.apply(intent);
        }
        self.window.wait(16);
        (answer, output)
    }

    /// Presses keys, one frame each.
    fn keys(&mut self, keys: &[NamedKey]) {
        for named in keys {
            let _ = self.step(&[key(*named, Modifiers::NONE)]);
        }
    }

    /// Whether the context menu of the rows is open.
    fn is_row_menu_open(&mut self) -> bool {
        self.step_and(&[], |ui| ui.is_menu_open("row menu")).0
    }
}

/// A click of the button of context menus at a point.
fn secondary_click(x: f32, y: f32) -> Vec<InputEvent> {
    let event = PointerButtonEvent {
        button: PointerButton::Secondary,
        position: render::Point::new(x, y),
        modifiers: Modifiers::NONE,
        click_count: 1,
    };
    vec![
        move_to(x, y),
        InputEvent::PointerPressed(event),
        InputEvent::PointerReleased(event),
    ]
}

#[test]
fn a_context_menu_opens_at_the_pointer_and_a_click_chooses_its_item() {
    let mut gallery = Gallery::new();
    let _ = gallery.step(&secondary_click(ROW.0, ROW.1));
    assert_eq!(gallery.state.menu_row, Some(ROW_NUMBER));
    assert!(gallery.is_row_menu_open());
    let output = gallery.step(&click(ROW.0 + 60.0, ROW.1 + 20.0));
    let copied = output.clipboard.expect("the hash is copied");
    assert_eq!(copied.len(), 7);
    assert_eq!(gallery.state.status, Some(format!("Copied {copied}")));
    assert!(!gallery.is_row_menu_open());
}

#[test]
fn the_keys_go_through_a_menu_and_into_its_submenu_and_escape_closes_one_level() {
    let mut gallery = Gallery::new();
    let _ = gallery.step(&secondary_click(ROW.0, ROW.1));
    let down = NamedKey::ArrowDown;
    gallery.keys(&[down, down, down, down, NamedKey::ArrowRight, down]);
    assert!(gallery.is_row_menu_open());
    gallery.keys(&[NamedKey::Escape]);
    assert!(gallery.is_row_menu_open(), "Escape closes the submenu only");
    gallery.keys(&[NamedKey::ArrowRight, NamedKey::End, NamedKey::Enter]);
    assert!(!gallery.is_row_menu_open());
    let status = gallery.state.status.clone().unwrap();
    assert!(
        status.starts_with("Reset main to ") && status.ends_with(", hard"),
        "{status}"
    );
}

#[test]
fn escape_closes_a_menu_and_a_click_outside_too() {
    let mut gallery = Gallery::new();
    let _ = gallery.step(&secondary_click(ROW.0, ROW.1));
    gallery.keys(&[NamedKey::Escape]);
    assert!(!gallery.is_row_menu_open());
    let _ = gallery.step(&secondary_click(ROW.0, ROW.1));
    assert!(gallery.is_row_menu_open());
    let _ = gallery.step(&click(100.0, 650.0));
    assert!(!gallery.is_row_menu_open());
    assert_eq!(gallery.state.status, None, "nothing was chosen");
}

#[test]
fn the_menu_of_a_button_opens_under_it_and_a_letter_goes_to_an_item() {
    let mut gallery = Gallery::new();
    let _ = gallery.step(&click(REPOSITORY.0, REPOSITORY.1));
    let _ = gallery.step(&[shortcut('t', Modifiers::NONE)]);
    gallery.keys(&[
        NamedKey::ArrowRight,
        NamedKey::ArrowDown,
        NamedKey::ArrowDown,
        NamedKey::Enter,
    ]);
    assert_eq!(gallery.state.theme, ThemeName::Light);
}

#[test]
fn a_dialog_takes_what_is_typed_and_enter_presses_its_main_button() {
    let mut gallery = Gallery::new();
    gallery
        .state
        .apply(&gallery::GalleryIntent::Open(Overlay::CreateBranch));
    let _ = gallery.step(&[]);
    let _ = gallery.step(&[typed("feature/graph")]);
    assert_eq!(gallery.state.new_branch, "feature/graph");
    gallery.keys(&[NamedKey::Enter]);
    assert_eq!(gallery.state.overlay, None);
    assert_eq!(gallery.state.status.as_deref(), Some("Created feature/graph"));
}

#[test]
fn tab_stays_inside_a_dialog_and_escape_cancels_it() {
    let mut gallery = Gallery::new();
    gallery
        .state
        .apply(&gallery::GalleryIntent::Open(Overlay::CreateBranch));
    let _ = gallery.step(&[]);
    let outside = gallery::widget_id("draft message");
    for _ in 0..6 {
        gallery.keys(&[NamedKey::Tab]);
        let focus = gallery.window.memory.focus();
        assert!(
            focus.is_some() && focus != Some(outside),
            "the focus stays in the dialog"
        );
    }
    gallery.keys(&[NamedKey::Escape]);
    assert_eq!(gallery.state.overlay, None);
    assert_eq!(gallery.state.status, None);
    let _ = gallery.step(&[]);
    assert_eq!(
        gallery.window.memory.focus(),
        Some(outside),
        "the focus goes back where it was"
    );
}

#[test]
fn a_confirmation_runs_on_enter_and_is_cancelled_by_escape() {
    let mut gallery = Gallery::new();
    gallery
        .state
        .apply(&gallery::GalleryIntent::Open(Overlay::ConfirmDelete));
    let _ = gallery.step(&[]);
    gallery.keys(&[NamedKey::Escape]);
    assert_eq!(gallery.state.overlay, None);
    assert_eq!(gallery.state.status, None);
    gallery
        .state
        .apply(&gallery::GalleryIntent::Open(Overlay::ConfirmDelete));
    let _ = gallery.step(&[]);
    gallery.keys(&[NamedKey::Enter]);
    assert_eq!(gallery.state.status.as_deref(), Some("Deleted feature/login-page"));
}

#[test]
fn ctrl_p_opens_the_picker_whose_filter_puts_the_best_match_first() {
    let mut gallery = Gallery::new();
    let _ = gallery.step(&[shortcut('p', Modifiers::CONTROL)]);
    assert_eq!(gallery.state.overlay, Some(Overlay::PickBranch));
    let _ = gallery.step(&[]);
    let _ = gallery.step(&[typed("srch")]);
    let _ = gallery.step(&[]);
    gallery.keys(&[NamedKey::Enter]);
    assert_eq!(gallery.state.status.as_deref(), Some("Switched to feature/search"));
}

#[test]
fn a_picker_of_several_checks_rows_with_ctrl_space_and_escape_cancels_it() {
    let mut gallery = Gallery::new();
    let _ = gallery.step(&[shortcut('p', Modifiers::CONTROL | Modifiers::SHIFT)]);
    assert_eq!(gallery.state.overlay, Some(Overlay::PickBranches));
    let _ = gallery.step(&[]);
    let _ = gallery.step(&[typed("fe")]);
    let toggle = shortcut(' ', Modifiers::CONTROL);
    let _ = gallery.step(std::slice::from_ref(&toggle));
    gallery.keys(&[NamedKey::ArrowDown]);
    let _ = gallery.step(&[toggle]);
    gallery.keys(&[NamedKey::Enter]);
    assert_eq!(
        gallery.state.status.as_deref(),
        Some("Showing feature/login-page, feature/search")
    );
    let _ = gallery.step(&[shortcut('p', Modifiers::CONTROL)]);
    let _ = gallery.step(&[]);
    gallery.keys(&[NamedKey::Escape]);
    assert_eq!(gallery.state.overlay, None);
}

/// Where the pill of main is, on the merge in the timeline.
const BADGE: (f32, f32) = (600.0, 281.0);

#[test]
fn the_pill_of_a_branch_dragged_onto_the_timeline_is_dropped_there() {
    let mut gallery = Gallery::new();
    let _ = gallery.step(&[move_to(BADGE.0, BADGE.1), press(BADGE.0, BADGE.1)]);
    let _ = gallery.step(&[move_to(BADGE.0 + 1.0, BADGE.1)]);
    let (dragged, _) = gallery.step_and(&[], |ui| ui.dragged());
    assert_eq!(dragged, None, "a move of a pixel is not a drag");
    let (dragged, _) = gallery.step_and(&[move_to(ROW.0, ROW.1)], |ui| ui.dragged());
    assert!(dragged.is_some());
    let _ = gallery.step(&[release(ROW.0, ROW.1)]);
    assert_eq!(gallery.state.status.as_deref(), Some("Dropped main on the timeline"));
}

#[test]
fn escape_cancels_a_drag() {
    let mut gallery = Gallery::new();
    let _ = gallery.step(&[move_to(BADGE.0, BADGE.1), press(BADGE.0, BADGE.1)]);
    let _ = gallery.step(&[move_to(ROW.0, ROW.1)]);
    let (dragged, _) = gallery.step_and(&[key(NamedKey::Escape, Modifiers::NONE)], |ui| ui.dragged());
    assert_eq!(dragged, None);
    let _ = gallery.step(&[release(ROW.0, ROW.1)]);
    assert_eq!(gallery.state.status, None, "nothing was dropped");
}

#[test]
fn ctrl_k_and_a_click_on_quick_search_open_it_and_escape_closes_it() {
    let mut gallery = Gallery::new();
    let _ = gallery.step(&[shortcut('k', Modifiers::CONTROL)]);
    assert_eq!(gallery.state.overlay, Some(Overlay::QuickSearch));
    let _ = gallery.step(&[]);
    gallery.keys(&[NamedKey::Escape]);
    assert_eq!(gallery.state.overlay, None);
    let _ = gallery.step(&[]);
    let _ = gallery.step(&click(700.0, 28.0));
    assert_eq!(gallery.state.overlay, Some(Overlay::QuickSearch));
    let _ = gallery.step(&[]);
    let _ = gallery.step(&[typed("login.rs")]);
    let _ = gallery.step(&[]);
    gallery.keys(&[NamedKey::Enter]);
    assert_eq!(gallery.state.status.as_deref(), Some("Opened src/login/page.rs"));
}

#[test]
fn the_activity_button_opens_the_drawer_and_escape_closes_it() {
    let mut gallery = Gallery::new();
    let _ = gallery.step(&click(1248.0, 28.0));
    assert!(gallery.state.is_activity_open);
    let _ = gallery.step(&[]);
    gallery.keys(&[NamedKey::Escape]);
    assert!(!gallery.state.is_activity_open);
}

#[test]
fn commit_with_an_empty_message_opens_a_dialog() {
    let mut gallery = Gallery::new();
    gallery
        .state
        .apply(&gallery::GalleryIntent::Typed(gallery::Field::Message, String::new()));
    let _ = gallery.step(&[]);
    let _ = gallery.step(&click(880.0, 110.0));
    assert_eq!(gallery.state.overlay, Some(Overlay::EmptyMessage));
    let _ = gallery.step(&[]);
    gallery.keys(&[NamedKey::Enter]);
    assert_eq!(gallery.state.overlay, None);
}
