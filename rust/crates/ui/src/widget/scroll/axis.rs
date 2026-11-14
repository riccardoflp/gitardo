//! How far something is scrolled along one axis: where it is shown now and where it is going, gliding there in a
//! short time so that a turn of the wheel does not jump.
//!
//! Offsets are `f64`: a list of a million rows is tens of millions of pixels long, past where `f32` counts single
//! pixels.

use std::time::Duration;

/// How long a glide to a new offset takes.
pub(crate) const GLIDE: Duration = Duration::from_millis(100);

/// How long to wait between the frames of a glide.
pub(crate) const GLIDE_FRAME: Duration = Duration::from_millis(16);

/// The offset of one axis of something scrolled, in logical pixels from the start of its content.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollAxis {
    /// The offset shown now.
    offset: f64,
    /// The offset it glides to.
    target: f64,
    /// The offset the glide started from.
    from: f64,
    /// When the glide started.
    started: Duration,
}

impl ScrollAxis {
    /// The offset shown now.
    #[must_use]
    pub fn offset(&self) -> f64 {
        self.offset
    }

    /// The offset it glides to, which is the offset shown once the glide is over.
    #[must_use]
    pub fn target(&self) -> f64 {
        self.target
    }

    /// Goes to an offset at once.
    pub(crate) fn jump(&mut self, offset: f64) {
        self.offset = offset;
        self.target = offset;
        self.from = offset;
    }

    /// Glides by a distance from where it is going.
    pub(crate) fn scroll_by(&mut self, distance: f64, now: Duration) {
        self.glide_to(self.target + distance, now);
    }

    /// Glides to an offset from the one shown now. The glide counts as started a frame ago, so that the frame of the
    /// turn of the wheel already moves, and a wheel turned at every frame keeps the content moving.
    pub(crate) fn glide_to(&mut self, target: f64, now: Duration) {
        self.from = self.offset;
        self.target = target;
        self.started = now.saturating_sub(GLIDE_FRAME);
    }

    /// Keeps the offsets between zero and the largest offset, where the end of the content meets the end of the view.
    pub(crate) fn clamp(&mut self, largest: f64) {
        let largest = largest.max(0.0);
        self.target = self.target.clamp(0.0, largest);
        self.offset = self.offset.clamp(0.0, largest);
        self.from = self.from.clamp(0.0, largest);
    }

    /// Moves the offset shown along the glide to the time of a frame, and says whether it is still gliding.
    pub(crate) fn advance(&mut self, now: Duration) -> bool {
        let elapsed = now.saturating_sub(self.started);
        if elapsed >= GLIDE || (self.target - self.offset).abs() < 0.01 {
            self.offset = self.target;
            self.from = self.target;
            return false;
        }
        let progress = elapsed.as_secs_f64() / GLIDE.as_secs_f64();
        let eased = 1.0 - (1.0 - progress).powi(3);
        self.offset = self.from + (self.target - self.from) * eased;
        true
    }

    /// Scrolls the least that brings a span into a view of a length: to its start when it is before, to its end when
    /// it is after, at once.
    pub(crate) fn reveal(&mut self, (start, end): (f64, f64), view: f64) {
        if start < self.target {
            self.jump(start);
        } else if end > self.target + view {
            self.jump(end - view);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scroll_glides_to_its_target_and_stays_inside_the_content() {
        let mut axis = ScrollAxis::default();
        axis.scroll_by(300.0, Duration::from_millis(1000));
        axis.clamp(200.0);
        assert_eq!(axis.target(), 200.0);
        assert!(axis.advance(Duration::from_millis(1000)));
        assert!(
            axis.offset() > 50.0 && axis.offset() < 100.0,
            "the frame of the turn already moves"
        );
        assert!(axis.advance(Duration::from_millis(1034)));
        assert!(axis.offset() > 150.0 && axis.offset() < 200.0);
        assert!(!axis.advance(Duration::from_millis(1084)));
        assert_eq!(axis.offset(), 200.0);
        axis.scroll_by(-500.0, Duration::from_millis(2000));
        axis.clamp(200.0);
        assert_eq!(axis.target(), 0.0);
    }

    #[test]
    fn revealing_a_span_scrolls_the_least() {
        let mut axis = ScrollAxis::default();
        axis.jump(100.0);
        axis.reveal((150.0, 170.0), 100.0);
        assert_eq!(axis.offset(), 100.0);
        axis.reveal((210.0, 230.0), 100.0);
        assert_eq!(axis.offset(), 130.0);
        axis.reveal((20.0, 40.0), 100.0);
        assert_eq!(axis.offset(), 20.0);
    }
}
