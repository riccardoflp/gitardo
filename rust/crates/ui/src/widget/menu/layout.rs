//! Where the levels of a menu go: at the pointer or under a button, and each submenu beside the item that opened it,
//! turned to the other side when there is no room, and always inside the window.

use render::{Point, Rect, Theme};

/// What a level of a menu is placed against.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Placement {
    /// A point, such as where the pointer was: the menu opens below it and to its right.
    At(Point),
    /// A button or another rectangle: the menu opens below it, its left sides lined up.
    Below(Rect),
    /// The item of a level that opens a submenu: the submenu opens to the right of the level, its first item level
    /// with the item.
    Beside {
        /// The item.
        item: Rect,
        /// The level the item is in.
        level: Rect,
    },
}

/// The sizes the parts of a menu take, from the tokens of the theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MenuMetrics {
    /// The height of an item.
    pub(crate) item: f32,
    /// The height of a separator.
    pub(crate) separator: f32,
    /// The room above the first item and below the last.
    pub(crate) padding: f32,
    /// The room left of the icons and right of the shortcuts.
    pub(crate) side: f32,
    /// The width kept for icons, so that the labels line up whether an item has one or not.
    pub(crate) icon_column: f32,
    /// The room between a label and its shortcut.
    pub(crate) shortcut_gap: f32,
    /// The narrowest a level is.
    pub(crate) least_width: f32,
}

impl MenuMetrics {
    /// The sizes of a menu in a theme.
    pub(crate) fn new(theme: &Theme) -> MenuMetrics {
        let sizes = theme.sizes;
        MenuMetrics {
            item: sizes.row_height + sizes.spacing.small,
            separator: sizes.spacing.medium + sizes.border,
            padding: sizes.spacing.small + sizes.spacing.tiny,
            side: sizes.spacing.large + sizes.spacing.tiny,
            icon_column: sizes.icon + sizes.spacing.medium,
            shortcut_gap: sizes.spacing.huge,
            least_width: sizes.row_height * 8.0,
        }
    }
}

/// The room kept between the foot of a menu under a button and the foot of the window, in logical pixels.
const GAP_BELOW: f32 = 8.0;

/// Where a level of a menu goes and how much of it is shown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MenuPlace {
    /// Where the level is drawn, inside the window.
    pub(crate) rect: Rect,
    /// The height of all of its entries with their padding: taller than [`MenuPlace::rect`] when it scrolls.
    pub(crate) content: f32,
}

impl MenuPlace {
    /// Whether the level is shorter than its entries, and scrolls.
    pub(crate) fn scrolls(self) -> bool {
        self.content > self.rect.height + 0.5
    }
}

/// Where a level of a size goes, inside the window, and how much of it is shown. `padding` is the room above the first
/// item of a submenu, which moves the submenu up so that its first item is level with the item that opened it.
///
/// A menu under a button opens below it and never above it, as tall as the room down to the foot of the window less a
/// gap; a menu at the pointer opens below it, else above it, else in the taller of the two rooms; a submenu opens
/// beside its item, moved up only as much as the window needs. What does not fit scrolls.
///
/// Feature: ui.menu-scroll
pub(crate) fn place(size: (f32, f32), placement: Placement, window: Rect, padding: f32) -> MenuPlace {
    let width = size.0.min(window.width);
    let content = size.1;
    let (x, (y, height)) = match placement {
        Placement::At(point) => (
            before_or_after(point.x, point.x, width, (window.x, window.right())),
            at_the_pointer(point.y, content, window),
        ),
        Placement::Below(rect) => {
            let room = (window.bottom() - GAP_BELOW - rect.bottom()).max(0.0);
            (
                before_or_after(rect.x, rect.right(), width, (window.x, window.right())),
                (rect.bottom(), content.min(room)),
            )
        }
        Placement::Beside { item, level } => {
            let height = content.min(window.height);
            (
                before_or_after(level.right(), level.x, width, (window.x, window.right())),
                ((item.y - padding).min(window.bottom() - height), height),
            )
        }
    };
    let x = x.clamp(window.x, (window.right() - width).max(window.x));
    let y = y.clamp(window.y, (window.bottom() - height).max(window.y));
    MenuPlace {
        rect: Rect::new(x, y, width, height),
        content,
    }
}

/// Where a menu opened at the pointer starts down and how tall it is: below the pointer when it fits there, else above
/// it when it fits there, else in the taller of the two rooms.
fn at_the_pointer(pointer: f32, content: f32, window: Rect) -> (f32, f32) {
    let below = window.bottom() - pointer;
    let above = pointer - window.y;
    if content <= below {
        (pointer, content)
    } else if content <= above {
        (pointer - content, content)
    } else if below >= above {
        (pointer, below.max(0.0))
    } else {
        (window.y, above.max(0.0))
    }
}

/// Where a length starts along an axis: from `after` onward when it fits before the end, otherwise ending at
/// `before`, which turns it to the other side.
fn before_or_after(after: f32, before: f32, length: f32, (start, end): (f32, f32)) -> f32 {
    if after + length <= end || before - length < start {
        after
    } else {
        before - length
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::*;

    const WINDOW: Rect = Rect::new(0.0, 0.0, 400.0, 300.0);

    fn rect_of(size: (f32, f32), placement: Placement) -> Rect {
        place(size, placement, WINDOW, 4.0).rect
    }

    #[test]
    fn a_menu_at_the_pointer_opens_below_and_right_and_turns_where_there_is_no_room() {
        let size = (100.0, 80.0);
        assert_eq!(
            rect_of(size, Placement::At(Point::new(10.0, 20.0))),
            Rect::new(10.0, 20.0, 100.0, 80.0)
        );
        assert_eq!(
            rect_of(size, Placement::At(Point::new(350.0, 250.0))),
            Rect::new(250.0, 170.0, 100.0, 80.0)
        );
        let tall = place((100.0, 500.0), Placement::At(Point::new(10.0, 20.0)), WINDOW, 4.0);
        assert_eq!(tall.rect, Rect::new(10.0, 20.0, 100.0, 280.0));
        assert!(tall.scrolls());
        let low = place((100.0, 500.0), Placement::At(Point::new(10.0, 260.0)), WINDOW, 4.0);
        assert_eq!(low.rect, Rect::new(10.0, 0.0, 100.0, 260.0), "the taller room is above");
    }

    #[test]
    fn a_submenu_opens_beside_its_item_and_turns_left_at_the_edge() {
        let level = Rect::new(10.0, 10.0, 100.0, 100.0);
        let item = Rect::new(10.0, 40.0, 100.0, 20.0);
        assert_eq!(
            rect_of((120.0, 60.0), Placement::Beside { item, level }),
            Rect::new(110.0, 36.0, 120.0, 60.0)
        );
        let level = Rect::new(250.0, 200.0, 100.0, 100.0);
        let item = Rect::new(250.0, 270.0, 100.0, 20.0);
        assert_eq!(
            rect_of((120.0, 60.0), Placement::Beside { item, level }),
            Rect::new(130.0, 240.0, 120.0, 60.0)
        );
        let tall = place((120.0, 900.0), Placement::Beside { item, level }, WINDOW, 4.0);
        assert_eq!(
            tall.rect,
            Rect::new(130.0, 0.0, 120.0, 300.0),
            "as tall as the window, and it scrolls"
        );
        assert!(tall.scrolls());
    }

    #[test]
    fn ui_menu_scroll__opened_under_a_button_in_a_window_400_high_it_ends_8_pixels_above_the_foot() {
        let window = Rect::new(0.0, 0.0, 1280.0, 400.0);
        let button = Rect::new(1200.0, 12.0, 32.0, 32.0);
        let placed = place((260.0, 1000.0), Placement::Below(button), window, 4.0);
        assert_eq!(placed.rect.y, button.bottom(), "its top at the foot of the button");
        assert_eq!(placed.rect.bottom(), window.bottom() - 8.0);
        assert!(placed.scrolls());
        assert_eq!(
            placed.rect.right(),
            button.right(),
            "turned left at the edge of the window"
        );
    }

    #[test]
    fn ui_menu_scroll__a_menu_that_fits_does_not_scroll_and_opens_where_it_opened_before() {
        let top = Rect::new(10.0, 10.0, 60.0, 20.0);
        let placed = place((100.0, 80.0), Placement::Below(top), WINDOW, 4.0);
        assert_eq!(placed.rect, Rect::new(10.0, 30.0, 100.0, 80.0));
        assert!(!placed.scrolls());
        let low = Rect::new(330.0, 250.0, 60.0, 20.0);
        let under = place((100.0, 80.0), Placement::Below(low), WINDOW, 4.0);
        assert_eq!(under.rect.y, low.bottom(), "never above its button");
        assert!(under.scrolls());
    }

    #[test]
    fn ui_menu_scroll__a_context_menu_near_the_foot_of_the_window_opens_above_the_pointer() {
        let placed = place((100.0, 120.0), Placement::At(Point::new(40.0, 280.0)), WINDOW, 4.0);
        assert_eq!(placed.rect, Rect::new(40.0, 160.0, 100.0, 120.0));
        assert!(!placed.scrolls());
    }
}
