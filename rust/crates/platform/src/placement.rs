//! Where a window is when it is restored, its size then and whether it is maximized: what the program keeps for the
//! next start, and whether a place kept still lands on a screen.

use crate::geometry::{LogicalSize, PhysicalSize, ScreenPosition};

/// How far into a screen, in physical pixels, the top left corner of a window must land for the window to be placed
/// there: enough of its title bar to take it with the pointer.
const REACHABLE: i32 = 48;

/// Where a window is and how large, as it is restored from maximized or minimized.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WindowPlacement {
    /// Where the top left corner of its frame is on the screen; none on a system that does not say, such as Wayland.
    pub position: Option<ScreenPosition>,
    /// The size of its inside, in logical pixels.
    pub size: LogicalSize,
    /// Whether it is maximized.
    pub is_maximized: bool,
}

/// A screen: where its top left corner is, and its size, in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Screen {
    /// Its top left corner.
    pub(crate) position: ScreenPosition,
    /// Its size.
    pub(crate) size: PhysicalSize,
}

/// Whether a window placed with its top left corner at a position can be taken with the pointer on one of the screens:
/// a place kept from a screen no longer there is not used.
pub(crate) fn lands_on_a_screen(position: ScreenPosition, screens: &[Screen]) -> bool {
    let x = i64::from(position.x) + i64::from(REACHABLE);
    let y = i64::from(position.y) + i64::from(REACHABLE / 2);
    screens.iter().any(|screen| {
        let left = i64::from(screen.position.x);
        let top = i64::from(screen.position.y);
        let right = left + i64::from(screen.size.width);
        let bottom = top + i64::from(screen.size.height);
        (left..right).contains(&x) && (top..bottom).contains(&y)
    })
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

    use super::*;

    /// A screen of 1920 by 1080 at the origin, and one of 2560 by 1440 to its left.
    fn two_screens() -> [Screen; 2] {
        [
            Screen {
                position: ScreenPosition { x: 0, y: 0 },
                size: PhysicalSize {
                    width: 1920,
                    height: 1080,
                },
            },
            Screen {
                position: ScreenPosition { x: -2560, y: -200 },
                size: PhysicalSize {
                    width: 2560,
                    height: 1440,
                },
            },
        ]
    }

    #[test]
    fn window_title_bar__a_place_kept_is_used_on_a_screen_and_not_off_every_screen() {
        let screens = two_screens();
        for kept in [
            ScreenPosition { x: 100, y: 80 },
            ScreenPosition { x: -2000, y: -100 },
            ScreenPosition { x: -8, y: -8 },
        ] {
            assert!(lands_on_a_screen(kept, &screens), "{kept:?}");
        }
        for gone in [
            ScreenPosition { x: 3000, y: 80 },
            ScreenPosition { x: 1900, y: 80 },
            ScreenPosition { x: 100, y: 1070 },
            ScreenPosition { x: -2000, y: 1300 },
        ] {
            assert!(!lands_on_a_screen(gone, &screens), "{gone:?}");
        }
        assert!(!lands_on_a_screen(ScreenPosition { x: 0, y: 0 }, &[]));
    }
}
