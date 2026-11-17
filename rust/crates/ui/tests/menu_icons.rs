//! The icons of the items of a menu, drawn in memory (`ui.menu-icons`): the words of an item without an icon start
//! where those of an item with one do, and an icon takes the color of the words of its item, dimmed with them when the
//! item cannot be chosen.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]
#![allow(
    clippy::cast_possible_truncation,
    reason = "the pixels of a small window are far fewer than an i32 counts"
)]

mod support;

use render::{Color, Icon, Point, ThemeName};
use support::Window;
use ui::{MenuAnchor, MenuEntry, MenuItem, Ui};

/// Where the menus open, in logical pixels.
const AT: Point = Point::new(40.0, 40.0);

/// The scale the menus are drawn at, so that a line of an icon covers whole pixels.
const SCALE: u8 = 2;

/// Opens a menu of one item at [`AT`] and draws it, in a window of its own.
fn drawn(item: MenuItem<'static, u8>) -> Window {
    let mut window = Window::new(ThemeName::Dark, (320, 160), SCALE);
    let entries = [MenuEntry::Item(item)];
    let view = |ui: &mut Ui<'_>| ui.menu("menu", &entries);
    let _ = window.frame(&[], |ui| {
        ui.open_menu("menu", MenuAnchor::At(AT));
        view(ui)
    });
    let _ = window.frame(&[], view);
    window
}

/// The rows of physical pixels of the item, and the columns inside the border of the menu, from its left.
fn item_area(window: &Window) -> (std::ops::Range<i32>, std::ops::Range<i32>) {
    let sizes = window.theme.sizes;
    let scale = f32::from(SCALE);
    let item = sizes.row_height + sizes.spacing.small;
    let top = AT.y + sizes.border + sizes.spacing.small + sizes.spacing.tiny;
    let rows = (top * scale) as i32 + 1..((top + item) * scale) as i32 - 1;
    let columns = ((AT.x + sizes.border) * scale) as i32 + 2..((AT.x + 200.0) * scale) as i32;
    (rows, columns)
}

/// How far a color is from another, the three channels added.
fn distance(one: Color, other: Color) -> u32 {
    let channel = |a: u8, b: u8| u32::from(a.abs_diff(b));
    channel(one.red, other.red) + channel(one.green, other.green) + channel(one.blue, other.blue)
}

/// The first column of the item, from the left, where something is drawn on the ground of the menu.
fn first_drawn(window: &Window, from: i32) -> Option<i32> {
    let ground = window.theme.colors.menu_background;
    let (rows, columns) = item_area(window);
    columns.filter(|column| *column >= from).find(|column| {
        rows.clone().any(|row| {
            window
                .frame
                .pixel(*column, row)
                .is_some_and(|pixel| distance(pixel, ground) > 24)
        })
    })
}

/// The color drawn farthest from the ground of the menu between two columns of the item.
fn strongest(window: &Window, columns: std::ops::Range<i32>) -> Color {
    let ground = window.theme.colors.menu_background;
    let (rows, _) = item_area(window);
    columns
        .flat_map(|column| rows.clone().map(move |row| (column, row)))
        .filter_map(|(column, row)| window.frame.pixel(column, row))
        .max_by_key(|pixel| distance(*pixel, ground))
        .unwrap()
}

#[test]
fn ui_menu_icons__the_words_of_an_item_without_an_icon_start_where_those_of_an_item_with_one_do() {
    let with = drawn(MenuItem::new("Words", 0).with_icon(Icon::Copy));
    let without = drawn(MenuItem::new("Words", 0));
    let (_, columns) = item_area(&without);
    let words = first_drawn(&without, columns.start).unwrap();
    let icon = first_drawn(&with, columns.start).unwrap();
    assert!(icon < words - 8, "the icon is left of the words: {icon} and {words}");
    let (rows, columns) = item_area(&with);
    for row in rows {
        for column in words - 2..columns.end {
            assert_eq!(
                with.frame.pixel(column, row),
                without.frame.pixel(column, row),
                "the words are drawn at the same place, at ({column}, {row})"
            );
        }
    }
}

#[test]
fn ui_menu_icons__an_icon_is_drawn_in_the_color_of_its_words_dimmed_with_them_when_its_item_cannot_be_chosen() {
    let colors = Window::new(ThemeName::Dark, (1, 1), 1).theme.colors;
    assert!(distance(colors.menu_foreground, colors.disabled_foreground) > 48);
    for (item, words) in [
        (MenuItem::new("Words", 0).with_icon(Icon::Copy), colors.menu_foreground),
        (
            MenuItem::new("Words", 0).with_icon(Icon::Copy).disabled(),
            colors.disabled_foreground,
        ),
    ] {
        let window = drawn(item);
        let without = drawn(MenuItem { icon: None, ..item });
        let (_, columns) = item_area(&window);
        let text = first_drawn(&without, columns.start).unwrap();
        let icon = strongest(&window, columns.start..text - 2);
        assert!(distance(icon, words) <= 24, "the icon is {icon:?}, the words {words:?}");
    }
}
