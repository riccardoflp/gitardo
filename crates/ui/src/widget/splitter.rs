//! Splitters: a rectangle cut in two panes by a line that the pointer drags to give one pane more room, never less than
//! each pane's least size; a double click on the line puts it back where it started.

use std::hash::Hash;

use render::Rect;

use super::metrics;
use crate::frame::Ui;
use crate::id::WidgetId;
use crate::input::{Cursor, PointerButton};
use crate::interaction::{Sense, WidgetEvent};
use crate::memory::DragStart;

/// How the two panes of a splitter sit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Split {
    /// Side by side, with a line between them from top to bottom.
    Columns,
    /// One above the other, with a line between them from left to right.
    Rows,
}

/// A splitter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Splitter {
    /// How the panes sit.
    pub split: Split,
    /// The size of the first pane before the line is moved, and after a double click on it, in logical pixels.
    pub default_size: f32,
    /// The least size of the first pane.
    pub least_first: f32,
    /// The least size of the second pane.
    pub least_second: f32,
}

/// What a splitter tells its view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitResponse {
    /// The first pane: the left one, or the top one.
    pub first: Rect,
    /// The second pane.
    pub second: Rect,
    /// The size of the first pane.
    pub size: f32,
    /// Whether the line moved in this frame, so that a view that keeps the size can keep the new one.
    pub is_changed: bool,
}

/// The size of the first pane kept inside the least sizes of both, in a length that holds the line too. When the
/// length cannot hold both least sizes, the first pane keeps its own.
pub(crate) fn clamp_size(size: f32, length: f32, splitter: &Splitter, line: f32) -> f32 {
    let room = (length - line).max(0.0);
    let largest = (room - splitter.least_second).max(splitter.least_first);
    size.clamp(splitter.least_first, largest).min(room).max(0.0)
}

impl Ui<'_> {
    /// Declares a splitter over a rectangle and returns its two panes, for the view to draw in.
    pub fn splitter(&mut self, key: impl Hash, rect: Rect, splitter: &Splitter) -> SplitResponse {
        let id = self.id(key);
        let theme = *self.theme();
        let line = theme.sizes.border;
        let length = match splitter.split {
            Split::Columns => rect.width,
            Split::Rows => rect.height,
        };
        let kept = self.memory_view().splitters.get(&id).copied();
        let mut size = clamp_size(kept.unwrap_or(splitter.default_size), length, splitter, line);
        let events = self.take_events(id);
        let before = size;
        for event in &events {
            size = self.move_splitter(id, event, (splitter, length, line), size);
        }
        let is_changed = (size - before).abs() > f32::EPSILON;
        let grip = metrics::splitter_grip(&theme);
        let grip_rect = line_rect(rect, splitter.split, size + line * 0.5 - grip * 0.5, grip);
        let response = self.interact_raised(id, grip_rect, (Sense::DRAG | Sense::CLICK, &events));
        if response.hovered || response.pressed {
            self.set_cursor(match splitter.split {
                Split::Columns => Cursor::ResizeEastWest,
                Split::Rows => Cursor::ResizeNorthSouth,
            });
        }
        let color = if response.hovered || response.pressed {
            theme.colors.focus_ring
        } else {
            theme.colors.border
        };
        self.fill_rect(line_rect(rect, splitter.split, size, line), color);
        let (first, second) = panes(rect, splitter.split, size, line);
        SplitResponse {
            first,
            second,
            size,
            is_changed,
        }
    }
}

impl Ui<'_> {
    /// Applies one event to the size of the first pane of a splitter: a press starts a drag, or puts the line back
    /// where it started when it is the second of a double click, and a drag moves the line.
    fn move_splitter(
        &mut self,
        id: WidgetId,
        event: &WidgetEvent,
        (splitter, length, line): (&Splitter, f32, f32),
        size: f32,
    ) -> f32 {
        match event {
            WidgetEvent::Pressed {
                button: PointerButton::Primary,
                position,
                click_count,
                ..
            } => {
                let (position, click_count) = (*position, *click_count);
                if click_count == 2 {
                    let _ = self.memory().splitters.remove(&id);
                    return clamp_size(splitter.default_size, length, splitter, line);
                }
                let value = f64::from(size);
                self.memory().drag = Some(DragStart { id, position, value });
                size
            }
            WidgetEvent::Dragged { position, .. } => {
                let Some(start) = self.memory_view().drag.filter(|drag| drag.id == id) else {
                    return size;
                };
                let moved = match splitter.split {
                    Split::Columns => position.x - start.position.x,
                    Split::Rows => position.y - start.position.y,
                };
                let moved_to = clamp_size(crate::number::narrow(start.value) + moved, length, splitter, line);
                let _ = self.memory().splitters.insert(id, moved_to);
                moved_to
            }
            _ => size,
        }
    }
}

/// A band across a rectangle at a distance from its start, of a thickness: the line of a splitter, or its grip.
fn line_rect(rect: Rect, split: Split, at: f32, thickness: f32) -> Rect {
    match split {
        Split::Columns => Rect::new(rect.x + at, rect.y, thickness, rect.height),
        Split::Rows => Rect::new(rect.x, rect.y + at, rect.width, thickness),
    }
}

/// The two panes of a rectangle split after a size, with a line between them.
fn panes(rect: Rect, split: Split, size: f32, line: f32) -> (Rect, Rect) {
    match split {
        Split::Columns => (
            Rect::new(rect.x, rect.y, size, rect.height),
            Rect::new(
                rect.x + size + line,
                rect.y,
                (rect.width - size - line).max(0.0),
                rect.height,
            ),
        ),
        Split::Rows => (
            Rect::new(rect.x, rect.y, rect.width, size),
            Rect::new(
                rect.x,
                rect.y + size + line,
                rect.width,
                (rect.height - size - line).max(0.0),
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_stays_between_the_least_sizes_of_both_panes() {
        let splitter = Splitter {
            split: Split::Columns,
            default_size: 200.0,
            least_first: 100.0,
            least_second: 150.0,
        };
        assert_eq!(clamp_size(200.0, 501.0, &splitter, 1.0), 200.0);
        assert_eq!(clamp_size(50.0, 501.0, &splitter, 1.0), 100.0);
        assert_eq!(clamp_size(450.0, 501.0, &splitter, 1.0), 350.0);
        assert_eq!(clamp_size(450.0, 201.0, &splitter, 1.0), 100.0);
        assert_eq!(clamp_size(450.0, 51.0, &splitter, 1.0), 50.0);
        let (first, second) = panes(Rect::new(0.0, 0.0, 501.0, 10.0), Split::Columns, 200.0, 1.0);
        assert_eq!((first.width, second.x, second.width), (200.0, 201.0, 300.0));
    }
}
