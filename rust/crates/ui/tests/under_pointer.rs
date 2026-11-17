//! What a row shows only under the pointer, over several frames with synthetic input: the pointer resting on a button a
//! row shows under it keeps the button and the background of the row, and a click presses the button; what lies over
//! the row on another layer, or across the rows, takes the pointer from it; and the rows of a list do the same.

#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]

mod support;

use render::{Color, Rect, ThemeName};
use support::{Window, move_to, press, release, wheel};
use ui::{Button, Layer, List, Response, SelectionMode, Sense, Ui};

/// Where the row is.
const ROW: Rect = Rect::new(10.0, 10.0, 300.0, 26.0);
/// Where the button of the row is, at its right end.
const BUTTON: Rect = Rect::new(282.0, 12.0, 22.0, 22.0);
/// A point of the row at the left of its button.
const ON_ROW: (f32, f32) = (40.0, 23.0);
/// A point of the button.
const ON_BUTTON: (f32, f32) = (293.0, 23.0);

/// A row with the background of a row under the pointer, and a button it shows only while the pointer is on it.
fn row_with_button(ui: &mut Ui<'_>) -> (Response, Option<Response>) {
    let id = ui.id("row");
    let row = ui.interact(id, ROW, Sense::CLICK);
    if row.contains_pointer {
        let color = ui.theme().colors.list_hover;
        ui.fill_rect(ROW, color);
    }
    let button = row
        .contains_pointer
        .then(|| ui.button("close", BUTTON, &Button::secondary("Close")));
    (row, button)
}

/// The color of the row, at a point of it no text and no button is drawn on.
fn row_color(window: &Window) -> Option<Color> {
    window.frame.pixel(20, 23)
}

/// A window with the row drawn once and the pointer then moved onto it, so that its button is there.
fn window_with_the_pointer_on_the_row() -> Window {
    let mut window = Window::new(ThemeName::Dark, (400, 200), 1);
    let _ = window.frame(&[], row_with_button);
    let ((row, button), _) = window.frame(&[move_to(ON_ROW.0, ON_ROW.1)], row_with_button);
    assert!(
        row.hovered && row.contains_pointer && button.is_some(),
        "the row under the pointer shows its button"
    );
    window
}

#[test]
fn the_pointer_resting_on_a_button_a_row_shows_under_it_keeps_the_button_and_a_click_presses_it() {
    let mut window = window_with_the_pointer_on_the_row();
    let lit = row_color(&window);
    assert_eq!(lit, Some(window.theme.colors.list_hover));
    for frame in 0..10 {
        let events = if frame == 0 {
            vec![move_to(ON_BUTTON.0, ON_BUTTON.1)]
        } else {
            Vec::new()
        };
        let ((row, button), output) = window.frame(&events, row_with_button);
        assert!(button.is_some(), "the button is drawn in frame {frame}");
        assert!(!row.hovered, "the button is the topmost widget under the pointer");
        assert_eq!(row_color(&window), lit, "the row keeps its background in frame {frame}");
        assert!(
            frame == 0 || output.dirty.is_empty(),
            "nothing changes in frame {frame}"
        );
    }
    let ((_, button), _) = window.frame(&[press(ON_BUTTON.0, ON_BUTTON.1)], row_with_button);
    assert!(
        button.is_some_and(|button| button.pressed),
        "the press is on the button"
    );
    let ((row, button), _) = window.frame(&[release(ON_BUTTON.0, ON_BUTTON.1)], row_with_button);
    assert!(button.is_some_and(|button| button.clicked), "the click is the button's");
    assert!(!row.clicked, "and not the row's");
}

#[test]
fn a_press_on_the_row_keeps_its_button_under_the_pointer_and_a_press_elsewhere_lights_no_row() {
    let mut window = window_with_the_pointer_on_the_row();
    let _ = window.frame(&[press(ON_ROW.0, ON_ROW.1)], row_with_button);
    for frame in 0..4 {
        let ((row, button), _) = window.frame(&[move_to(ON_BUTTON.0, ON_BUTTON.1)], row_with_button);
        assert!(
            row.contains_pointer && row.pressed,
            "the row holds the press in frame {frame}"
        );
        assert!(button.is_some_and(|button| !button.hovered && !button.pressed));
    }
    let ((row, button), _) = window.frame(&[release(ON_BUTTON.0, ON_BUTTON.1)], row_with_button);
    assert!(!row.clicked && button.is_some_and(|button| !button.clicked));
    let _ = window.frame(&[move_to(350.0, 150.0), press(350.0, 150.0)], row_with_button);
    let ((row, button), _) = window.frame(&[move_to(ON_ROW.0, ON_ROW.1)], row_with_button);
    assert!(
        row.contains_pointer && button.is_some(),
        "a press on no widget holds nothing"
    );
    let _ = window.frame(&[release(ON_ROW.0, ON_ROW.1)], row_with_button);
    let _ = window.frame(&[move_to(ON_BUTTON.0, ON_BUTTON.1)], row_with_button);
    let _ = window.frame(&[press(ON_BUTTON.0, ON_BUTTON.1)], row_with_button);
    let ((row, _), _) = window.frame(&[move_to(350.0, 150.0)], row_with_button);
    assert!(!row.contains_pointer, "the pointer left the row");
    let ((row, button), _) = window.frame(&[move_to(ON_ROW.0, ON_ROW.1)], row_with_button);
    assert!(
        !row.contains_pointer && !row.hovered && button.is_none(),
        "a drag that started on another widget lights nothing it passes over"
    );
}

/// The row and its button under a menu on the layer of menus, which covers the left half of the row, and a bar that
/// lies across the row on its own layer, as the grip of a splitter does.
fn row_under_a_menu_and_a_bar(ui: &mut Ui<'_>) -> (Response, Response, Response) {
    let (row, _) = row_with_button(ui);
    let bar_id = ui.id("bar");
    let bar = ui.interact_above(bar_id, Rect::new(200.0, 0.0, 6.0, 200.0), Sense::CLICK);
    let menu = ui.layer(Layer::Popup, |ui| {
        ui.button("item", Rect::new(0.0, 0.0, 150.0, 100.0), &Button::secondary("Item"))
    });
    (row, bar, menu)
}

#[test]
fn what_lies_over_a_row_on_another_layer_or_across_it_takes_the_pointer_from_the_row() {
    let mut window = Window::new(ThemeName::Dark, (400, 200), 1);
    let _ = window.frame(&[], row_under_a_menu_and_a_bar);
    let ((row, _, menu), _) = window.frame(&[move_to(ON_ROW.0, ON_ROW.1)], row_under_a_menu_and_a_bar);
    assert!(menu.hovered && menu.contains_pointer);
    assert!(!row.contains_pointer, "a menu over the row is not drawn inside it");
    let ((row, bar, _), _) = window.frame(&[move_to(203.0, 23.0)], row_under_a_menu_and_a_bar);
    assert!(bar.hovered && bar.contains_pointer);
    assert!(!row.contains_pointer, "a bar across the row is not drawn inside it");
    let ((row, bar, menu), _) = window.frame(&[move_to(250.0, 23.0)], row_under_a_menu_and_a_bar);
    assert!(row.contains_pointer && row.hovered);
    assert!(!bar.contains_pointer && !menu.contains_pointer);
}

/// Where the list is.
const LIST: Rect = Rect::new(0.0, 0.0, 300.0, 100.0);
/// The height of a row of the list.
const ROW_HEIGHT: f32 = 20.0;

/// A list of a hundred rows, each with a button at its right end only while the pointer is on the row. Gives the row
/// under the pointer and what its button said.
fn list_with_buttons(ui: &mut Ui<'_>) -> Option<(usize, Response)> {
    let list = List {
        name: "Rows",
        row_count: 100,
        row_height: ROW_HEIGHT,
        selection_mode: SelectionMode::Single,
        lead: 0.0,
    };
    let mut under_pointer = None;
    let _ = ui.list("rows", LIST, &list, |ui, row| {
        if row.is_hovered {
            let place = Rect::new(row.rect.right() - 60.0, row.rect.y + 1.0, 40.0, row.rect.height - 2.0);
            let button = ui.button(("fold", row.index), place, &Button::secondary("Fold"));
            under_pointer = Some((row.index, button));
        }
    });
    under_pointer
}

#[test]
fn a_row_of_a_list_keeps_the_button_it_shows_under_the_pointer_and_the_scroll_bar_lights_no_row() {
    let mut window = Window::new(ThemeName::Dark, (400, 200), 1);
    let _ = window.frame(&[], list_with_buttons);
    let (under, _) = window.frame(&[move_to(100.0, 50.0)], list_with_buttons);
    assert_eq!(under.map(|(row, _)| row), Some(2));
    for frame in 0..10 {
        let events = if frame == 0 {
            vec![move_to(260.0, 50.0)]
        } else {
            Vec::new()
        };
        let (under, output) = window.frame(&events, list_with_buttons);
        let (row, button) = under.unwrap();
        assert_eq!(row, 2, "the row keeps its button in frame {frame}");
        assert!(button.hovered, "which is the widget under the pointer");
        assert!(
            frame == 0 || output.dirty.is_empty(),
            "nothing changes in frame {frame}"
        );
    }
    let _ = window.frame(&[press(260.0, 50.0)], list_with_buttons);
    let (under, _) = window.frame(&[release(260.0, 50.0)], list_with_buttons);
    assert!(under.is_some_and(|(row, button)| row == 2 && button.clicked));
    let (under, _) = window.frame(&[move_to(297.0, 50.0)], list_with_buttons);
    assert_eq!(under, None, "the scroll bar lies across the rows");
}

#[test]
fn a_list_scrolled_under_the_pointer_shows_the_button_of_the_row_that_comes_under_it() {
    let mut window = Window::new(ThemeName::Dark, (400, 200), 1);
    let _ = window.frame(&[], list_with_buttons);
    let _ = window.frame(&[move_to(260.0, 50.0)], list_with_buttons);
    let (under, _) = window.frame(&[], list_with_buttons);
    assert!(under.is_some_and(|(row, button)| row == 2 && button.hovered));
    let _ = window.frame(&[wheel(260.0, 50.0, 3.0)], list_with_buttons);
    window.wait(2000);
    let mut rows = Vec::new();
    for _ in 0..4 {
        let (under, _) = window.frame(&[], list_with_buttons);
        rows.push(under.map(|(row, _)| row));
    }
    let first = rows.first().copied().flatten().unwrap();
    assert!(first > 2, "another row came under the pointer: {rows:?}");
    assert!(
        rows.iter().all(|row| *row == Some(first)),
        "and it is lit in every frame: {rows:?}"
    );
}

/// A title over a rectangle, short of the room of the button it shows beside it only while the pointer is within the
/// rectangle, with a gap between the two that is neither; under a menu that covers the left end of the rectangle.
fn title_with_button(ui: &mut Ui<'_>) -> Option<Response> {
    let is_within = ui.is_pointer_within(ROW);
    let room = if is_within { 250.0 } else { ROW.width };
    let id = ui.id("title");
    let _ = ui.interact(id, Rect::new(ROW.x, ROW.y, room, ROW.height), Sense::CLICK);
    let button = is_within.then(|| ui.button("stash", BUTTON, &Button::secondary("Stash")));
    let _ = ui.layer(Layer::Popup, |ui| {
        ui.button("item", Rect::new(0.0, 0.0, 30.0, 100.0), &Button::secondary("Item"))
    });
    button
}

#[test]
fn a_button_shown_beside_a_title_under_the_pointer_stays_on_the_title_the_button_and_the_gap_between() {
    let mut window = Window::new(ThemeName::Dark, (400, 200), 1);
    let _ = window.frame(&[], title_with_button);
    for (x, place) in [(100.0, "the title"), (ON_BUTTON.0, "the button"), (270.0, "the gap")] {
        for frame in 0..4 {
            let events = if frame == 0 { vec![move_to(x, 23.0)] } else { Vec::new() };
            let (button, _) = window.frame(&events, title_with_button);
            assert!(
                button.is_some(),
                "the button is drawn with the pointer on {place}, in frame {frame}"
            );
        }
    }
    let (button, _) = window.frame(&[move_to(20.0, 23.0)], title_with_button);
    assert!(button.is_none(), "a menu over the title takes the pointer from it");
    let (button, _) = window.frame(&[move_to(100.0, 80.0)], title_with_button);
    assert!(button.is_none(), "the pointer left the title");
}
