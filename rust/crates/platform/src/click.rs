//! Counting clicks: a press soon after the last one, near it and with the same button, is a double click, then a
//! triple click, and so on.

use crate::input::PointerButton;

/// A press of a button, as the system reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Press {
    /// The button.
    pub(crate) button: PointerButton,
    /// Where, in physical pixels.
    pub(crate) x: i32,
    /// Where, in physical pixels.
    pub(crate) y: i32,
    /// When, in milliseconds of a clock that wraps around.
    pub(crate) time: u32,
}

/// How soon and how near a press must be to the last one to count as the next click.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ClickLimits {
    /// The longest time between the presses, in milliseconds.
    pub(crate) time: u32,
    /// The width of the rectangle around the first press that the next must fall in, in physical pixels.
    pub(crate) width: i32,
    /// Its height.
    pub(crate) height: i32,
}

/// The last press and how many clicks it made.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ClickCounter {
    last: Option<(Press, u32)>,
}

impl ClickCounter {
    /// How many clicks this press makes with the ones before it: 1, 2, 3…
    pub(crate) fn press(&mut self, press: Press, limits: ClickLimits) -> u32 {
        let count = match self.last {
            Some((last, count)) if continues(last, press, limits) => count.saturating_add(1),
            _ => 1,
        };
        self.last = Some((press, count));
        count
    }

    /// How many clicks the last press made, for its release.
    pub(crate) fn last_count(&self) -> u32 {
        self.last.map_or(1, |(_, count)| count)
    }
}

/// Whether a press continues the clicks of the last one.
fn continues(last: Press, press: Press, limits: ClickLimits) -> bool {
    last.button == press.button
        && press.time.wrapping_sub(last.time) <= limits.time
        && press.x.abs_diff(last.x) <= (limits.width / 2).unsigned_abs()
        && press.y.abs_diff(last.y) <= (limits.height / 2).unsigned_abs()
}

#[cfg(test)]
mod tests {
    use super::{ClickCounter, ClickLimits, Press};
    use crate::input::PointerButton;

    const LIMITS: ClickLimits = ClickLimits {
        time: 500,
        width: 4,
        height: 4,
    };

    fn press(button: PointerButton, x: i32, time: u32) -> Press {
        Press { button, x, y: 10, time }
    }

    #[test]
    fn counts_clicks() {
        let primary = PointerButton::Primary;
        let cases: [(&[Press], u32); 6] = [
            (&[press(primary, 10, 0)], 1),
            (&[press(primary, 10, 0), press(primary, 11, 400)], 2),
            (
                &[press(primary, 10, 0), press(primary, 10, 400), press(primary, 10, 800)],
                3,
            ),
            (&[press(primary, 10, 0), press(primary, 10, 600)], 1),
            (&[press(primary, 10, 0), press(primary, 13, 100)], 1),
            (&[press(primary, 10, 0), press(PointerButton::Secondary, 10, 100)], 1),
        ];
        for (presses, expected) in cases {
            let mut counter = ClickCounter::default();
            let counts: Vec<u32> = presses.iter().map(|given| counter.press(*given, LIMITS)).collect();
            assert_eq!(counts.last(), Some(&expected), "{presses:?}");
            assert_eq!(counter.last_count(), expected);
        }
    }

    #[test]
    fn counts_across_the_wrap_of_the_clock() {
        let mut counter = ClickCounter::default();
        let _ = counter.press(press(PointerButton::Primary, 0, u32::MAX - 100), LIMITS);
        assert_eq!(counter.press(press(PointerButton::Primary, 0, 100), LIMITS), 2);
    }
}
