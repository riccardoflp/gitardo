//! Menus and pickers longer than the window, driven over several frames with synthetic input: a menu under a button
//! in a short window scrolled by the wheel and the keys (`ui.menu-scroll`), and the list of a picker scrolled by its
//! bar and gone through by the keys (`ui.picker-scroll`).

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]

mod support;

use render::{Rect, ThemeName};
use support::{Window, click, key, move_to, press, release, wheel};
use ui::{Button, InputBox, InputEvent, MenuEntry, MenuItem, Modifiers, NamedKey, PickList, PickListResponse, Ui};

/// The button the menu opens under, near the top of the window.
const BUTTON: Rect = Rect::new(100.0, 12.0, 80.0, 32.0);
/// How many items the menu has: more than a window 400 pixels high shows.
const ITEMS: usize = 40;
/// Where the last item is once the menu is scrolled to its end: the menu ends 8 pixels above the foot of the window,
/// and the last item 7 pixels above that, inside its border and its padding, 30 pixels high.
const LAST_ITEM_Y: f32 = 400.0 - 8.0 - 7.0 - 15.0;

/// The labels of the items of the menu.
fn labels() -> Vec<String> {
    (0..ITEMS).map(|index| format!("Item {index}")).collect()
}

/// Draws the button and its menu, and returns the item chosen.
fn menu_view(ui: &mut Ui<'_>, labels: &[String]) -> Option<usize> {
    let entries: Vec<MenuEntry<'_, usize>> = labels
        .iter()
        .enumerate()
        .map(|(index, label)| MenuEntry::Item(MenuItem::new(label, index)))
        .collect();
    ui.menu_button("long menu", BUTTON, &Button::secondary("More"), &entries)
}

/// A window 400 pixels high with the menu open under its button.
fn opened() -> (Window, Vec<String>) {
    let labels = labels();
    let mut window = Window::new(ThemeName::Dark, (1280, 400), 1);
    let _ = window.frame(&[], |ui| menu_view(ui, &labels));
    let (chosen, _) = window.frame(&click(140.0, 28.0), |ui| menu_view(ui, &labels));
    assert_eq!(chosen, None);
    let (chosen, _) = window.frame(&[], |ui| menu_view(ui, &labels));
    assert_eq!(chosen, None);
    (window, labels)
}

#[test]
fn ui_menu_scroll__the_wheel_scrolls_the_menu_until_its_last_entry_is_shown_whole_and_no_further() {
    let (mut window, labels) = opened();
    let (chosen, _) = window.frame(&click(160.0, LAST_ITEM_Y), |ui| menu_view(ui, &labels));
    assert!(
        chosen.is_some_and(|item| item < 20),
        "at first the menu shows its first items: {chosen:?}"
    );
    let (mut window, labels) = opened();
    for _ in 0..20 {
        let _ = window.frame(&[wheel(160.0, 200.0, 3.0)], |ui| menu_view(ui, &labels));
        window.wait(16);
    }
    let (chosen, _) = window.frame(&click(160.0, LAST_ITEM_Y), |ui| menu_view(ui, &labels));
    assert_eq!(chosen, Some(ITEMS - 1), "the last entry is at the foot of the menu");
    let (mut window, labels) = opened();
    for _ in 0..40 {
        let _ = window.frame(&[wheel(160.0, 200.0, 3.0)], |ui| menu_view(ui, &labels));
    }
    let (chosen, _) = window.frame(&click(160.0, LAST_ITEM_Y - 30.0), |ui| menu_view(ui, &labels));
    assert_eq!(chosen, Some(ITEMS - 2), "no further than the last entry");
}

#[test]
fn ui_menu_scroll__end_lights_the_last_entry_and_shows_it_and_enter_chooses_it() {
    let (mut window, labels) = opened();
    let (chosen, _) = window.frame(&[key(NamedKey::End, Modifiers::NONE)], |ui| menu_view(ui, &labels));
    assert_eq!(chosen, None);
    let _ = window.frame(&[], |ui| menu_view(ui, &labels));
    let (chosen, _) = window.frame(&[key(NamedKey::Enter, Modifiers::NONE)], |ui| menu_view(ui, &labels));
    assert_eq!(chosen, Some(ITEMS - 1));
    let (mut window, labels) = opened();
    let _ = window.frame(&[key(NamedKey::End, Modifiers::NONE)], |ui| menu_view(ui, &labels));
    let _ = window.frame(&[], |ui| menu_view(ui, &labels));
    let (chosen, _) = window.frame(&click(160.0, LAST_ITEM_Y), |ui| menu_view(ui, &labels));
    assert_eq!(chosen, Some(ITEMS - 1), "the last entry is shown at the foot");
}

/// How many rows the list of the picker has, with the headings of two groups at 0 and 50.
const ROWS: usize = 100;
/// The list of the picker: ten rows of 26 pixels.
const LIST: Rect = Rect::new(10.0, 50.0, 300.0, 260.0);

/// The rows of the list: a heading at 0 and at 50.
fn headings() -> Vec<bool> {
    (0..ROWS).map(|index| index == 0 || index == 50).collect()
}

/// Draws the box of the picker and its list.
fn picker_view(ui: &mut Ui<'_>, headings: &[bool]) -> PickListResponse {
    let _ = ui.input_box("box", Rect::new(10.0, 10.0, 300.0, 32.0), &InputBox::new("", "Filter"));
    let list = PickList {
        headings,
        typed: "",
        keys_from: ui.id("box"),
        is_tall: false,
    };
    ui.pick_list("list", LIST, &list)
}

#[test]
fn ui_picker_scroll__dragging_the_bar_to_its_foot_shows_the_last_row() {
    let headings = headings();
    let mut window = Window::new(ThemeName::Dark, (400, 400), 1);
    let (shown, _) = window.frame(&[], |ui| picker_view(ui, &headings));
    assert_eq!(shown.rows.first().map(|(index, _)| *index), Some(0));
    let thumb = (LIST.right() - 5.0, LIST.y + 5.0);
    let drag: Vec<InputEvent> = vec![move_to(thumb.0, thumb.1), press(thumb.0, thumb.1)];
    let _ = window.frame(&drag, |ui| picker_view(ui, &headings));
    let _ = window.frame(&[move_to(thumb.0, 390.0)], |ui| picker_view(ui, &headings));
    let _ = window.frame(&[release(thumb.0, 390.0)], |ui| picker_view(ui, &headings));
    let (shown, _) = window.frame(&[], |ui| picker_view(ui, &headings));
    let (last, rect) = shown.rows.last().copied().unwrap();
    assert_eq!(last, ROWS - 1);
    assert!(rect.bottom() <= LIST.bottom() + 0.5, "shown whole: {rect:?}");
}

#[test]
fn ui_picker_scroll__the_arrows_never_light_a_heading() {
    let headings = headings();
    let mut window = Window::new(ThemeName::Dark, (400, 400), 1);
    let _ = window.frame(&[], |ui| picker_view(ui, &headings));
    let _ = window.frame(&click(100.0, 26.0), |ui| picker_view(ui, &headings));
    let (shown, _) = window.frame(&[key(NamedKey::ArrowUp, Modifiers::NONE)], |ui| {
        picker_view(ui, &headings)
    });
    assert_eq!(shown.lit, Some(1), "the heading above the first row is not lit");
    let mut lit = Vec::new();
    for _ in 0..60 {
        let (shown, _) = window.frame(&[key(NamedKey::ArrowDown, Modifiers::NONE)], |ui| {
            picker_view(ui, &headings)
        });
        lit.push(shown.lit.unwrap());
    }
    assert!(!lit.contains(&50), "{lit:?}");
    assert!(lit.contains(&49) && lit.contains(&51), "{lit:?}");
    let (shown, _) = window.frame(&[], |ui| picker_view(ui, &headings));
    let shown_rows: Vec<usize> = shown.rows.iter().map(|(index, _)| *index).collect();
    assert!(shown_rows.contains(&61), "the row lit is shown: {shown_rows:?}");
}
