//! The columns of a Merge page and the gutters between them: the base when it is shown, the current side, the result
//! and the incoming side, each as wide as the others.

use ui::{Rect, Ui};

/// Where the columns and the gutters are, across the rows of a page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Columns {
    /// The base and the gutter that joins it to the current side, when the base is shown.
    pub(crate) base: Option<(Rect, Rect)>,
    /// The current side.
    pub(crate) current: Rect,
    /// The gutter between the current side and the result, with the bands of the current side.
    pub(crate) left: Rect,
    /// The result.
    pub(crate) result: Rect,
    /// The gutter between the result and the incoming side, with the bands of the incoming side.
    pub(crate) right: Rect,
    /// The incoming side, up to the scroll bar.
    pub(crate) incoming: Rect,
}

impl Columns {
    /// The columns of the rows of a page over a rectangle, three or four as wide as each other.
    pub(crate) fn of(rect: Rect, is_base_shown: bool, ui: &Ui<'_>) -> Columns {
        let theme = ui.theme();
        let gutter = gutter_width(ui);
        let room = (rect.width - theme.sizes.scrollbar).max(0.0);
        let (count, gutters) = if is_base_shown { (4.0, 3.0) } else { (3.0, 2.0) };
        let width = ((room - gutters * gutter) / count).max(0.0);
        let mut x = rect.x;
        let mut next = |wide: f32| {
            let taken = Rect::new(x, rect.y, wide, rect.height);
            x += wide;
            taken
        };
        let base = is_base_shown.then(|| (next(width), next(gutter)));
        Columns {
            base,
            current: next(width),
            left: next(gutter),
            result: next(width),
            right: next(gutter),
            incoming: next(width),
        }
    }

    /// From the first column to the last.
    pub(crate) fn span(&self) -> (f32, f32) {
        let left = self.base.map_or(self.current.x, |(base, _)| base.x);
        (left, self.incoming.right())
    }
}

/// The width of a gutter: room for three small buttons side by side on the band of a region.
pub(crate) fn gutter_width(ui: &Ui<'_>) -> f32 {
    ui.theme().sizes.control_height * 2.0
}
