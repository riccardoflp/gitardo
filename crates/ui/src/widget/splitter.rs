//! Splitters: a rectangle cut in two panes by a line that the pointer drags to give one pane more room, never less than
//! each pane's least size; a double click on the line puts it back where it started, and with the keyboard on it the
//! arrows move it by a step.

use std::hash::Hash;

use render::Rect;

use super::metrics;
use crate::frame::Ui;
use crate::id::WidgetId;
use crate::input::{Cursor, Key, KeyEvent, NamedKey, PointerButton};
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
    /// The size of the first pane the view keeps, which the splitter takes over its own; none while the view keeps
    /// none, and then the splitter remembers the size it was moved to.
    pub size: Option<f32>,
    /// How far an arrow moves the line while the splitter has the keyboard; with 0 the splitter takes no keyboard.
    pub step: f32,
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
    /// Whether the line was put back where it started by a double click in this frame.
    pub is_reset: bool,
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
        let line = self.theme().sizes.border;
        self.splitter_across(key, rect, splitter, line)
    }

    /// A splitter whose panes are apart by a gap rather than a line, such as the islands of a window: the gap is
    /// where it is dragged, and a line lights up in its middle while the pointer or the keyboard is on it.
    pub fn splitter_across(&mut self, key: impl Hash, rect: Rect, splitter: &Splitter, line: f32) -> SplitResponse {
        let id = self.id(key);
        let theme = *self.theme();
        let length = match splitter.split {
            Split::Columns => rect.width,
            Split::Rows => rect.height,
        };
        let kept = splitter.size.or_else(|| self.memory_view().splitters.get(&id).copied());
        let mut size = clamp_size(kept.unwrap_or(splitter.default_size), length, splitter, line);
        let events = self.take_events(id);
        let before = size;
        let mut is_reset = false;
        for event in &events {
            let moved = self.move_splitter(id, event, (splitter, length, line), size);
            is_reset |= moved.is_reset;
            size = moved.size;
        }
        let is_changed = (size - before).abs() > f32::EPSILON || is_reset;
        let grip = metrics::splitter_grip(&theme);
        let grip = grip.max(line);
        let grip_rect = line_rect(rect, splitter.split, size + line * 0.5 - grip * 0.5, grip);
        let sense = if splitter.step > 0.0 {
            Sense::DRAG | Sense::CLICK | Sense::FOCUS
        } else {
            Sense::DRAG | Sense::CLICK
        };
        let response = self.interact_raised(id, grip_rect, (sense, &events));
        if response.hovered || response.pressed {
            self.set_cursor(match splitter.split {
                Split::Columns => Cursor::ResizeEastWest,
                Split::Rows => Cursor::ResizeNorthSouth,
            });
        }
        // The line shows only while the pointer or the keyboard is on it: the parts of the window are told apart by
        // their tones.
        if response.hovered || response.pressed || response.shows_focus {
            let border = theme.sizes.border;
            let lit = line_rect(rect, splitter.split, size + line * 0.5 - 1.5 * border, 3.0 * border);
            self.fill_rect(lit, theme.colors.focus_ring);
        }
        let (first, second) = panes(rect, splitter.split, size, line);
        SplitResponse {
            first,
            second,
            size,
            is_changed,
            is_reset,
        }
    }
}

/// Where an event left the line of a splitter.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Moved {
    /// The size of the first pane.
    size: f32,
    /// Whether a double click put it back where it started.
    is_reset: bool,
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
    ) -> Moved {
        let at = |size| Moved { size, is_reset: false };
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
                    return Moved {
                        size: clamp_size(splitter.default_size, length, splitter, line),
                        is_reset: true,
                    };
                }
                let value = f64::from(size);
                self.memory().drag = Some(DragStart { id, position, value });
                at(size)
            }
            WidgetEvent::Dragged { position, .. } => {
                let Some(start) = self.memory_view().drag.filter(|drag| drag.id == id) else {
                    return at(size);
                };
                let moved = match splitter.split {
                    Split::Columns => position.x - start.position.x,
                    Split::Rows => position.y - start.position.y,
                };
                let moved_to = clamp_size(crate::number::narrow(start.value) + moved, length, splitter, line);
                let _ = self.memory().splitters.insert(id, moved_to);
                at(moved_to)
            }
            WidgetEvent::Key(key) => {
                let Some(direction) = arrow_direction(splitter, key) else {
                    return at(size);
                };
                self.claim_key(id, *key);
                let moved_to = clamp_size(size + direction * splitter.step, length, splitter, line);
                let _ = self.memory().splitters.insert(id, moved_to);
                at(moved_to)
            }
            _ => at(size),
        }
    }
}

/// Which way an arrow moves the line of a splitter that takes the keyboard: -1 back, 1 on; none for another key.
fn arrow_direction(splitter: &Splitter, key: &KeyEvent) -> Option<f32> {
    if splitter.step <= 0.0 || !key.modifiers.is_empty() {
        return None;
    }
    match (splitter.split, key.key) {
        (Split::Columns, Key::Named(NamedKey::ArrowLeft)) | (Split::Rows, Key::Named(NamedKey::ArrowUp)) => Some(-1.0),
        (Split::Columns, Key::Named(NamedKey::ArrowRight)) | (Split::Rows, Key::Named(NamedKey::ArrowDown)) => {
            Some(1.0)
        }
        _ => None,
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
            size: None,
            step: 0.0,
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
