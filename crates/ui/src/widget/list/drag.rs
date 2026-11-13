//! A row of a list dragged: the list notes the row the main button is pressed on, and says once that a drag starts
//! from it when the pointer has moved far enough, so that its view can say what is dragged (`Ui::start_drag`). A
//! click and a double click on the row stay what they are.

use render::{Point, Rect};

use super::{List, row_at};
use crate::frame::Ui;
use crate::input::PointerButton;
use crate::interaction::WidgetEvent;
use crate::memory::ListState;

/// The row the main button is held on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RowPress {
    /// Which row.
    row: usize,
    /// Where the button was pressed, in logical pixels of the window.
    at: Point,
    /// Whether the list said already that a drag starts from it.
    is_said: bool,
}

/// A drag that starts from a row of a list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowDrag {
    /// The row.
    pub row: usize,
    /// Where the button was pressed on it, in logical pixels of the window: what is under that point is what is
    /// dragged, such as a label of the row.
    pub at: Point,
}

impl Ui<'_> {
    /// Notes the press and the release of the main button on a row of a list, and gives the drag that starts from the
    /// row once the pointer moved far enough from the press.
    pub(super) fn row_drag(
        &mut self,
        (rect, list): (Rect, &List<'_>),
        state: &mut ListState,
        event: &WidgetEvent,
    ) -> Option<RowDrag> {
        match event {
            WidgetEvent::Pressed {
                button: PointerButton::Primary,
                position,
                ..
            } => {
                state.press = row_at(list, state.scroll.offset(), position.y - rect.y).map(|row| RowPress {
                    row,
                    at: *position,
                    is_said: false,
                });
                None
            }
            WidgetEvent::Released {
                button: PointerButton::Primary,
                ..
            } => {
                state.press = None;
                None
            }
            WidgetEvent::Dragged { position, .. } => {
                let threshold = self.theme().sizes.spacing.small;
                let press = state.press.as_mut().filter(|press| !press.is_said)?;
                let moved = Point::new(position.x - press.at.x, position.y - press.at.y);
                (moved.length() > threshold).then(|| {
                    press.is_said = true;
                    RowDrag {
                        row: press.row,
                        at: press.at,
                    }
                })
            }
            _ => None,
        }
    }
}
