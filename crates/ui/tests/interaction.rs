//! Interaction over several frames with synthetic input: clicks, the focus and Tab, keyboard activation, layers that
//! take the pointer first, a modal layer, hover and tooltips.

#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]

mod support;

use render::{Rect, ThemeName};
use support::{Window, click, key, move_to, press, release};
use ui::{Button, Cursor, InputEvent, Key, KeyEvent, Layer, Modifiers, NamedKey, Response, Ui, WidgetId};

/// Three buttons in a row, 100 pixels wide, at 10, 120 and 230 across, 10 down.
fn three_buttons(ui: &mut Ui<'_>) -> [Response; 3] {
    ["first", "second", "third"].map(|name| {
        let index = match name {
            "first" => 0.0,
            "second" => 1.0,
            _ => 2.0,
        };
        let rect = Rect::new(10.0 + 110.0 * index, 10.0, 100.0, 26.0);
        ui.button(name, rect, &Button::secondary(name).with_tooltip("Does what it says"))
    })
}

/// A window with the three buttons drawn once, so that the next frame's input finds them.
fn window() -> Window {
    let mut window = Window::new(ThemeName::Dark, (400, 200), 1);
    let _ = window.frame(&[], three_buttons);
    window
}

#[test]
fn a_click_needs_the_press_and_the_release_on_the_same_widget() {
    let mut window = window();
    let (pressed, _) = window.frame(&[move_to(20.0, 20.0), press(20.0, 20.0)], three_buttons);
    assert!(pressed[0].pressed && !pressed[0].clicked);
    let (moved, _) = window.frame(&[move_to(130.0, 20.0), release(130.0, 20.0)], three_buttons);
    assert!(moved.iter().all(|response| !response.clicked));
    let (clicked, _) = window.frame(&click(140.0, 20.0), three_buttons);
    assert_eq!(clicked.map(|response| response.clicked), [false, true, false]);
    let (outside, _) = window.frame(&click(390.0, 190.0), three_buttons);
    assert!(outside.iter().all(|response| !response.clicked && !response.has_focus));
}

#[test]
fn tab_goes_round_the_widgets_in_order_and_enter_or_space_clicks() {
    let mut window = window();
    let tab = key(NamedKey::Tab, Modifiers::NONE);
    let back = key(NamedKey::Tab, Modifiers::SHIFT);
    let focused = |responses: &[Response; 3]| responses.iter().position(|response| response.has_focus);
    let (after, _) = window.frame(std::slice::from_ref(&tab), three_buttons);
    assert_eq!(focused(&after), Some(0));
    assert!(after[0].shows_focus);
    let (after, _) = window.frame(&[tab.clone(), tab.clone()], three_buttons);
    assert_eq!(focused(&after), Some(2));
    let (after, _) = window.frame(std::slice::from_ref(&tab), three_buttons);
    assert_eq!(focused(&after), Some(0));
    let (after, _) = window.frame(std::slice::from_ref(&back), three_buttons);
    assert_eq!(focused(&after), Some(2));
    let (after, _) = window.frame(&[key(NamedKey::Enter, Modifiers::NONE)], three_buttons);
    assert!(after[2].clicked);
    let space = ui::InputEvent::KeyPressed(KeyEvent::press(Key::Character(' '), Modifiers::NONE));
    let (after, _) = window.frame(&[space], three_buttons);
    assert!(after[2].clicked);
    let (after, _) = window.frame(&[key(NamedKey::Escape, Modifiers::NONE)], three_buttons);
    assert!(after[2].escaped && !after[2].clicked);
}

#[test]
fn a_click_gives_the_focus_without_a_ring_and_a_widget_gone_loses_it() {
    let mut window = window();
    let (after, _) = window.frame(&click(20.0, 20.0), three_buttons);
    assert!(after[0].has_focus && !after[0].shows_focus);
    let _ = window.frame(&[], |ui| ui.window());
    assert_eq!(window.memory.focus(), None);
}

/// A button on the base layer under a popup with a button of its own, declared before the base button.
fn popup_over_button(ui: &mut Ui<'_>) -> (Response, Response) {
    let popup = ui.layer(Layer::Popup, |ui| {
        ui.button("item", Rect::new(0.0, 0.0, 100.0, 40.0), &Button::secondary("Item"))
    });
    let base = ui.button("base", Rect::new(0.0, 0.0, 200.0, 100.0), &Button::secondary("Base"));
    (popup, base)
}

#[test]
fn a_higher_layer_takes_the_pointer_first_whatever_the_order_of_declaration() {
    let mut window = window();
    let _ = window.frame(&[], popup_over_button);
    let ((popup, base), _) = window.frame(&click(10.0, 10.0), popup_over_button);
    assert!(popup.clicked && !base.clicked);
    let ((popup, base), _) = window.frame(&click(150.0, 80.0), popup_over_button);
    assert!(!popup.clicked && base.clicked);
}

/// Two buttons on the base layer and a dialog with one button.
fn dialog(ui: &mut Ui<'_>) -> ([Response; 3], Response) {
    let base = three_buttons(ui);
    let dialog = ui.layer(Layer::Modal, |ui| {
        ui.button("ok", Rect::new(100.0, 100.0, 80.0, 26.0), &Button::primary("OK"))
    });
    (base, dialog)
}

#[test]
fn a_modal_layer_keeps_the_pointer_and_the_keyboard_from_the_layers_below() {
    let mut window = window();
    let _ = window.frame(&[], dialog);
    let ((base, _), _) = window.frame(&click(20.0, 20.0), dialog);
    assert!(!base[0].clicked);
    let tab = key(NamedKey::Tab, Modifiers::NONE);
    let ((base, ok), _) = window.frame(&[tab.clone(), tab], dialog);
    assert!(ok.has_focus && base.iter().all(|response| !response.has_focus));
    let ((_, ok), _) = window.frame(&click(110.0, 110.0), dialog);
    assert!(ok.clicked);
}

#[test]
fn the_tooltip_shows_after_the_pointer_stays_and_a_frame_is_asked_for_then() {
    let mut window = window();
    window.wait(1000);
    let (hovered, output) = window.frame(&[move_to(20.0, 20.0)], three_buttons);
    assert!(hovered[0].hovered);
    let due = output.next_frame.unwrap();
    assert_eq!(due, window.now + std::time::Duration::from_millis(600));
    let background = window.theme.colors.tooltip_background;
    window.wait(300);
    let (_, early) = window.frame(&[], three_buttons);
    assert_eq!(early.next_frame, Some(due));
    assert_ne!(window.frame.pixel(26, 46), Some(background));
    window.now = due;
    let (_, shown) = window.frame(&[], three_buttons);
    assert!(!shown.dirty.is_empty(), "the tooltip is drawn");
    assert_eq!(
        window.frame.pixel(26, 46),
        Some(background),
        "the tooltip is drawn under the pointer"
    );
    let (_, again) = window.frame(&[], three_buttons);
    assert!(again.dirty.is_empty(), "nothing changed, nothing is drawn");
    assert_eq!(again.cursor, Cursor::Arrow);
    let (_, gone) = window.frame(&[move_to(390.0, 190.0)], three_buttons);
    assert!(!gone.dirty.is_empty(), "the place of the tooltip is drawn again");
    assert_ne!(window.frame.pixel(26, 46), Some(background));
}

#[test]
fn the_keys_no_widget_used_are_left_for_the_shortcuts_of_the_view() {
    let mut window = window();
    let shortcut = key(NamedKey::Function(5), Modifiers::NONE);
    let (keys, _) = window.frame(std::slice::from_ref(&shortcut), |ui| {
        let _ = three_buttons(ui);
        (ui.unclaimed_keys().len(), ui.key_presses().len())
    });
    assert_eq!(keys, (1, 1));
    let _ = window.frame(&click(20.0, 20.0), three_buttons);
    let enter = key(NamedKey::Enter, Modifiers::NONE);
    let (keys, _) = window.frame(&[shortcut, enter], |ui| {
        let _ = three_buttons(ui);
        (ui.unclaimed_keys(), ui.key_presses().len())
    });
    assert_eq!(
        keys,
        (
            vec![KeyEvent::press(Key::Named(NamedKey::Function(5)), Modifiers::NONE)],
            2
        ),
        "the focused button uses Enter, not F5"
    );
    assert_eq!(window.memory.focus(), Some(WidgetId::ROOT.with("first")));
}

#[test]
fn a_click_hides_the_tooltip_until_the_pointer_leaves_the_widget() {
    let mut window = window();
    let background = window.theme.colors.tooltip_background;
    let _ = window.frame(&click(20.0, 20.0), three_buttons);
    window.wait(2000);
    let (_, output) = window.frame(&[], three_buttons);
    assert_eq!(output.next_frame, None, "no tooltip is waited for");
    assert_ne!(
        window.frame.pixel(26, 46),
        Some(background),
        "no tooltip after the click"
    );
    let _ = window.frame(&[move_to(390.0, 190.0)], three_buttons);
    let _ = window.frame(&[move_to(20.0, 20.0)], three_buttons);
    window.wait(2000);
    let _ = window.frame(&[], three_buttons);
    assert_eq!(
        window.frame.pixel(26, 46),
        Some(background),
        "back over the button, the tooltip shows again"
    );
}

/// An input box over the width of the window.
fn input_box(ui: &mut Ui<'_>) -> Vec<KeyEvent> {
    let input = ui::InputBox {
        text: "",
        placeholder: "Name",
        message: None,
    };
    let _ = ui.input_box("name", Rect::new(10.0, 10.0, 300.0, 26.0), &input);
    ui.unclaimed_keys()
}

#[test]
fn an_input_box_with_the_focus_leaves_the_keys_it_did_not_use_to_the_view() {
    let mut window = Window::new(ThemeName::Dark, (400, 200), 1);
    let _ = window.frame(&[], input_box);
    let _ = window.frame(&click(20.0, 20.0), input_box);
    let left = key(NamedKey::ArrowLeft, Modifiers::NONE);
    let letter = InputEvent::KeyPressed(KeyEvent::press(Key::Character('a'), Modifiers::NONE));
    let copy = InputEvent::KeyPressed(KeyEvent::press(Key::Character('c'), Modifiers::CONTROL));
    let refresh = key(NamedKey::Function(5), Modifiers::NONE);
    let palette = InputEvent::KeyPressed(KeyEvent::press(Key::Character('p'), Modifiers::CONTROL));
    let (unclaimed, _) = window.frame(&[left, letter, copy, refresh, palette], input_box);
    assert_eq!(
        unclaimed,
        vec![
            KeyEvent::press(Key::Named(NamedKey::Function(5)), Modifiers::NONE),
            KeyEvent::press(Key::Character('p'), Modifiers::CONTROL),
        ]
    );
}
