//! Scroll bars: a thin track along one side of what scrolls, with a thumb as long as the part of the content in view,
//! that can be dragged, and a click on the track that scrolls by a page.

use render::{Point, Rect};

use super::axis::ScrollAxis;
use crate::frame::Ui;
use crate::id::WidgetId;
use crate::input::PointerButton;
use crate::interaction::{Sense, WidgetEvent};
use crate::memory::DragStart;
use crate::widget::panel::Axis;

/// What a scroll bar shows: which way it runs, the length of the view and of the content.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BarSpec {
    /// Which way it runs.
    pub(crate) axis: Axis,
    /// The length of the view.
    pub(crate) view: f64,
    /// The length of the content.
    pub(crate) content: f64,
}

impl BarSpec {
    /// The largest offset: where the end of the content meets the end of the view.
    pub(crate) fn largest(self) -> f64 {
        (self.content - self.view).max(0.0)
    }
}

/// The thumb of a scroll bar on its track, for an offset; nothing when all of the content is in view.
pub(crate) fn thumb(track: Rect, spec: BarSpec, offset: f64, shortest: f32) -> Option<Rect> {
    if spec.content <= spec.view || spec.content <= 0.0 {
        return None;
    }
    let length = along(track, spec.axis);
    let thumb = (crate::number::narrow(f64::from(length) * spec.view / spec.content))
        .max(shortest)
        .min(length);
    let travel = f64::from(length - thumb);
    let start = crate::number::narrow(travel * (offset / spec.largest()).clamp(0.0, 1.0));
    Some(match spec.axis {
        Axis::Vertical => Rect::new(track.x, track.y + start, track.width, thumb),
        Axis::Horizontal => Rect::new(track.x + start, track.y, thumb, track.height),
    })
}

/// The length of a rectangle along an axis.
fn along(rect: Rect, axis: Axis) -> f32 {
    match axis {
        Axis::Vertical => rect.height,
        Axis::Horizontal => rect.width,
    }
}

/// The coordinate of a point along an axis.
fn coordinate(point: Point, axis: Axis) -> f32 {
    match axis {
        Axis::Vertical => point.y,
        Axis::Horizontal => point.x,
    }
}

impl Ui<'_> {
    /// Draws a scroll bar over a track and moves the offset as its thumb is dragged or its track clicked.
    pub(crate) fn scroll_bar(&mut self, id: WidgetId, track: Rect, spec: BarSpec, state: &mut ScrollAxis) {
        let shortest = self.theme().sizes.row_height;
        let Some(before) = thumb(track, spec, state.target(), shortest) else {
            return;
        };
        let (response, events) = self.interact_with_events(id, track, Sense::DRAG);
        let travel = f64::from(along(track, spec.axis) - along(before, spec.axis));
        let target = state.target();
        for event in events {
            match event {
                WidgetEvent::Pressed {
                    button: PointerButton::Primary,
                    position,
                    ..
                } => {
                    if before.contains(position) {
                        self.memory().drag = Some(DragStart {
                            id,
                            position,
                            value: state.target(),
                        });
                    } else {
                        let is_before =
                            coordinate(position, spec.axis) < coordinate(Point::new(before.x, before.y), spec.axis);
                        let page = if is_before { -spec.view } else { spec.view };
                        state.scroll_by(page, self.now());
                    }
                }
                WidgetEvent::Dragged { position, .. } => {
                    let Some(start) = self.memory_view().drag.filter(|drag| drag.id == id) else {
                        continue;
                    };
                    let moved = f64::from(coordinate(position, spec.axis) - coordinate(start.position, spec.axis));
                    let offset = if travel > 0.0 {
                        start.value + moved * spec.largest() / travel
                    } else {
                        start.value
                    };
                    state.jump(offset.clamp(0.0, spec.largest()));
                }
                _ => {}
            }
        }
        // What the bar moved was drawn at its old offset in this frame: the next frame draws it where it went.
        if (state.target() - target).abs() > f64::EPSILON {
            let now = self.now();
            self.request_frame_at(now);
        }
        let theme = *self.theme();
        let Some(shown) = thumb(track, spec, state.offset(), shortest) else {
            return;
        };
        let is_lit = response.hovered || response.pressed;
        let color = if is_lit {
            theme.colors.scrollbar_thumb_hover
        } else {
            theme.colors.scrollbar_thumb
        };
        let shape = thin(shown, spec.axis, theme.sizes.spacing, is_lit);
        let radius = shape.width.min(shape.height) * 0.5;
        self.fill_rounded_rect(shape, radius, color);
    }
}

/// The thumb as it is drawn: a thin rounded line along the far side of its track, a little wider under the pointer.
fn thin(thumb: Rect, axis: Axis, spacing: render::Spacing, is_lit: bool) -> Rect {
    let thickness = if is_lit {
        spacing.small + spacing.tiny
    } else {
        spacing.small
    };
    let edge = spacing.tiny;
    match axis {
        Axis::Vertical => Rect::new(
            thumb.right() - edge - thickness,
            thumb.y + edge,
            thickness,
            (thumb.height - 2.0 * edge).max(0.0),
        ),
        Axis::Horizontal => Rect::new(
            thumb.x + edge,
            thumb.bottom() - edge - thickness,
            (thumb.width - 2.0 * edge).max(0.0),
            thickness,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_thumb_is_as_long_as_the_view_and_moves_with_the_offset() {
        let track = Rect::new(0.0, 0.0, 10.0, 100.0);
        let spec = BarSpec {
            axis: Axis::Vertical,
            view: 100.0,
            content: 400.0,
        };
        assert_eq!(thumb(track, spec, 0.0, 20.0), Some(Rect::new(0.0, 0.0, 10.0, 25.0)));
        assert_eq!(thumb(track, spec, 300.0, 20.0), Some(Rect::new(0.0, 75.0, 10.0, 25.0)));
        assert_eq!(thumb(track, spec, 900.0, 20.0), Some(Rect::new(0.0, 75.0, 10.0, 25.0)));
        let long = BarSpec {
            content: 22_000_000.0,
            ..spec
        };
        assert_eq!(thumb(track, long, 0.0, 20.0), Some(Rect::new(0.0, 0.0, 10.0, 20.0)));
        assert_eq!(thumb(track, BarSpec { content: 50.0, ..spec }, 0.0, 20.0), None);
    }
}
