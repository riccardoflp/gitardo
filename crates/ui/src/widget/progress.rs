//! Progress: a bar that fills as work is done, and a spinner for work whose length is not known.

use std::f32::consts::TAU;
use std::time::Duration;

use render::{Color, LineCap, Path, Point, Rect, Stroke, Theme};

use super::{is_high_contrast, metrics};
use crate::frame::Ui;
use crate::layout::{Align, place};

/// How long the spinner takes to turn once.
const TURN: Duration = Duration::from_millis(960);

/// How many steps a turn of the spinner is drawn in: a frame for each, so that a waiting window does not draw at the
/// rate of the screen.
const STEPS_PER_TURN: u16 = 24;

/// How much of the circle the arc of the spinner covers.
const ARC: f32 = 0.7;

/// How many straight lines the arc of the spinner is drawn with.
const ARC_SEGMENTS: u16 = 24;

impl Ui<'_> {
    /// Draws a bar in the middle of a rectangle, down, filled from the left to a fraction from 0 to 1.
    pub fn progress_bar(&mut self, rect: Rect, fraction: f32) {
        let theme = *self.theme();
        let thickness = metrics::progress_thickness(&theme);
        let track = place(rect, (rect.width, thickness), (Align::Start, Align::Center));
        let radius = thickness * 0.5;
        self.fill_rounded_rect(track, radius, track_color(&theme));
        let fraction = if fraction.is_nan() {
            0.0
        } else {
            fraction.clamp(0.0, 1.0)
        };
        let filled = Rect::new(track.x, track.y, track.width * fraction, track.height);
        if filled.width > 0.0 {
            self.fill_rounded_rect(filled, radius, theme.colors.accent);
        }
    }

    /// Draws a spinner in the middle of a rectangle, turning with the time of the frame, and asks for the frame of its
    /// next step.
    pub fn spinner(&mut self, rect: Rect) {
        let theme = *self.theme();
        let side = theme.sizes.icon;
        let square = place(rect, (side, side), (Align::Center, Align::Center));
        let line = theme.sizes.spacing.tiny;
        let radius = (side - line) * 0.5;
        let center = square.center();
        let (step, next) = spinner_step(self.now());
        self.request_frame_at(next);
        let ring = Stroke {
            width: line,
            cap: LineCap::Butt,
            color: track_color(&theme),
        };
        self.stroke_circle(center, radius, ring);
        let start = f32::from(step) / f32::from(STEPS_PER_TURN) * TAU;
        let arc = arc_path(center, radius, start, ARC * TAU);
        let stroke = Stroke {
            width: line,
            cap: LineCap::Round,
            color: theme.colors.accent,
        };
        self.stroke_path(arc, stroke);
    }
}

/// The color of the part of a progress bar or a spinner not yet filled: the borders of the theme, or a grey in the
/// high contrast themes, whose borders have the color of what fills it.
fn track_color(theme: &Theme) -> Color {
    if is_high_contrast(theme) {
        theme.colors.disabled_foreground
    } else {
        theme.colors.border
    }
}

/// The step of its turn the spinner is at, at a time, and the time of the next step.
fn spinner_step(now: Duration) -> (u16, Duration) {
    let step_length = TURN.as_millis() / u128::from(STEPS_PER_TURN);
    let steps = crate::number::steps(now.as_millis(), step_length);
    let step = u16::try_from(steps % u128::from(STEPS_PER_TURN)).unwrap_or(0);
    let next = u64::try_from((steps + 1) * step_length).map_or(Duration::MAX, Duration::from_millis);
    (step, next)
}

/// An arc of a circle from an angle, clockwise, as straight lines.
pub(crate) fn arc_path(center: Point, radius: f32, start: f32, sweep: f32) -> Path {
    let mut path = Path::new();
    for segment in 0..=ARC_SEGMENTS {
        let angle = start + sweep * f32::from(segment) / f32::from(ARC_SEGMENTS);
        let point = Point::new(center.x + radius * angle.cos(), center.y + radius * angle.sin());
        let _ = if segment == 0 {
            path.move_to(point)
        } else {
            path.line_to(point)
        };
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spinner_moves_a_step_at_a_time_and_asks_for_the_next() {
        assert_eq!(spinner_step(Duration::ZERO), (0, Duration::from_millis(40)));
        assert_eq!(
            spinner_step(Duration::from_millis(1000)),
            (1, Duration::from_millis(1040))
        );
        assert_eq!(
            spinner_step(Duration::from_millis(999)),
            (0, Duration::from_millis(1000))
        );
    }
}
