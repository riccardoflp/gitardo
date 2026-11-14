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

/// Where a level of a size goes, inside the window. `padding` is the room above the first item of a submenu, which
/// moves the submenu up so that its first item is level with the item that opened it.
pub(crate) fn place(size: (f32, f32), placement: Placement, window: Rect, padding: f32) -> Rect {
    let (width, height) = (size.0.min(window.width), size.1.min(window.height));
    let (x, y) = match placement {
        Placement::At(point) => (
            before_or_after(point.x, point.x, width, (window.x, window.right())),
            before_or_after(point.y, point.y, height, (window.y, window.bottom())),
        ),
        Placement::Below(rect) => (
            before_or_after(rect.x, rect.right(), width, (window.x, window.right())),
            before_or_after(rect.bottom(), rect.y, height, (window.y, window.bottom())),
        ),
        Placement::Beside { item, level } => (
            before_or_after(level.right(), level.x, width, (window.x, window.right())),
            (item.y - padding).min(window.bottom() - height),
        ),
    };
    let x = x.clamp(window.x, (window.right() - width).max(window.x));
    let y = y.clamp(window.y, (window.bottom() - height).max(window.y));
    Rect::new(x, y, width, height)
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
mod tests {
    use super::*;

    const WINDOW: Rect = Rect::new(0.0, 0.0, 400.0, 300.0);

    #[test]
    fn a_menu_at_the_pointer_opens_below_and_right_and_turns_where_there_is_no_room() {
        let size = (100.0, 80.0);
        assert_eq!(
            place(size, Placement::At(Point::new(10.0, 20.0)), WINDOW, 4.0),
            Rect::new(10.0, 20.0, 100.0, 80.0)
        );
        assert_eq!(
            place(size, Placement::At(Point::new(350.0, 250.0)), WINDOW, 4.0),
            Rect::new(250.0, 170.0, 100.0, 80.0)
        );
        let tall = place((100.0, 500.0), Placement::At(Point::new(10.0, 20.0)), WINDOW, 4.0);
        assert_eq!(tall, Rect::new(10.0, 0.0, 100.0, 300.0));
    }

    #[test]
    fn a_menu_under_a_button_lines_up_with_it_or_opens_above() {
        let button = Rect::new(330.0, 270.0, 60.0, 20.0);
        assert_eq!(
            place((100.0, 80.0), Placement::Below(button), WINDOW, 4.0),
            Rect::new(290.0, 190.0, 100.0, 80.0)
        );
        let top = Rect::new(10.0, 10.0, 60.0, 20.0);
        assert_eq!(
            place((100.0, 80.0), Placement::Below(top), WINDOW, 4.0),
            Rect::new(10.0, 30.0, 100.0, 80.0)
        );
    }

    #[test]
    fn a_submenu_opens_beside_its_item_and_turns_left_at_the_edge() {
        let level = Rect::new(10.0, 10.0, 100.0, 100.0);
        let item = Rect::new(10.0, 40.0, 100.0, 20.0);
        assert_eq!(
            place((120.0, 60.0), Placement::Beside { item, level }, WINDOW, 4.0),
            Rect::new(110.0, 36.0, 120.0, 60.0)
        );
        let level = Rect::new(250.0, 200.0, 100.0, 100.0);
        let item = Rect::new(250.0, 270.0, 100.0, 20.0);
        assert_eq!(
            place((120.0, 60.0), Placement::Beside { item, level }, WINDOW, 4.0),
            Rect::new(130.0, 240.0, 120.0, 60.0)
        );
    }
}
