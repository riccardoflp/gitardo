//! The row of a list under the pointer: the one the pointer is over, or over something drawn inside it, such as the
//! arrow that folds a merge or the clock of a file, so that what a row shows only under the pointer stays there.

use render::Rect;

use super::{List, row_at};
use crate::frame::Ui;
use crate::interaction::Response;
use crate::number;

impl Ui<'_> {
    /// The row under the pointer, by the offset the list is drawn at now: while the list is the topmost widget under
    /// the pointer, and while the topmost is a widget drawn inside a row, such as a button the row shows under the
    /// pointer. That widget is where the last frame put it, so the row it is inside is looked for by the offset the
    /// list was last drawn at. None over the scroll bar and whatever else lies across the rows.
    pub(super) fn row_under_pointer(
        &self,
        (rect, list): (Rect, &List<'_>),
        response: &Response,
        (drawn, now): (f64, f64),
    ) -> Option<usize> {
        let point = self.pointer()?;
        if response.hovered {
            return row_at(list, now, point.y - rect.y);
        }
        let under = self
            .memory_view()
            .interaction
            .under
            .filter(|_| response.contains_pointer)?;
        let around = row_at(list, drawn, point.y - rect.y)?;
        let top = rect.y + number::narrow(number::wide_of_count(around) * f64::from(list.row_height) - drawn);
        let band = Rect::new(rect.x, top, rect.width, list.row_height);
        if under.is_inside(under.layer, band) {
            row_at(list, now, point.y - rect.y)
        } else {
            None
        }
    }
}
