//! The settings of the system the libraries of the window do not read, the same on every system: asked of Windows,
//! of the desktop's `gsettings` on Linux where it has them, of `AppKit` on macOS, and the usual values elsewhere.

use crate::click::ClickLimits;
#[cfg(not(windows))]
use crate::geometry::Scale;

/// What a notch of the wheel scrolls when the system says a page: Gitardo scrolls this many lines instead.
#[cfg(windows)]
pub(crate) const LINES_FOR_A_PAGE: f32 = 20.0;

/// How far a notch of the wheel scrolls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WheelSteps {
    /// In lines, down or up.
    pub(crate) down: f32,
    /// In characters, across.
    pub(crate) across: f32,
}

#[cfg(windows)]
pub(crate) use crate::windows::{click_limits, is_high_contrast, wheel_steps};

#[cfg(target_os = "linux")]
use crate::linux::double_click_time;
#[cfg(target_os = "linux")]
pub(crate) use crate::linux::is_high_contrast;

#[cfg(target_os = "macos")]
use crate::macos::double_click_time;
#[cfg(target_os = "macos")]
pub(crate) use crate::macos::is_high_contrast;

/// How far the local time of the computer is from UTC now, in seconds: what `${date}` in the names of new branches is
/// counted in.
#[must_use]
pub fn utc_offset() -> i64 {
    #[cfg(windows)]
    {
        crate::windows::utc_offset()
    }
    #[cfg(target_os = "linux")]
    {
        crate::linux::utc_offset()
    }
    #[cfg(target_os = "macos")]
    {
        crate::macos::utc_offset()
    }
    // Elsewhere the local time is UTC until the part of each system is written.
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        0
    }
}

/// Whether high contrast is on: not known yet on this system.
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
pub(crate) fn is_high_contrast() -> bool {
    false
}

/// How soon the next press must come to make a double click: the usual half second.
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn double_click_time() -> u32 {
    500
}

/// How soon and how near the next press must come to make a double click: as soon as the desktop says, and within
/// the usual 4 pixels.
#[cfg(not(windows))]
pub(crate) fn click_limits(scale: Scale) -> ClickLimits {
    let side = i32::try_from(crate::convert::round_to_pixels(4.0 * scale.factor())).unwrap_or(i32::MAX);
    ClickLimits {
        time: double_click_time(),
        width: side,
        height: side,
    }
}

/// How far a notch of the wheel scrolls: the usual 3 lines.
#[cfg(not(windows))]
pub(crate) fn wheel_steps() -> WheelSteps {
    WheelSteps { down: 3.0, across: 3.0 }
}

/// Makes sure the limits of a double click are usable: at least a moment and a pixel.
pub(crate) fn usable(limits: ClickLimits) -> ClickLimits {
    ClickLimits {
        time: limits.time.max(1),
        width: limits.width.max(1),
        height: limits.height.max(1),
    }
}

#[cfg(test)]
mod tests {
    use super::{click_limits, is_high_contrast, usable, wheel_steps};
    use crate::click::ClickLimits;
    use crate::geometry::Scale;

    #[test]
    fn reads_the_settings_of_the_system() {
        let _ = is_high_contrast();
        let limits = usable(click_limits(Scale::default()));
        assert!(
            limits.time >= 1 && limits.width >= 1 && limits.height >= 1,
            "{limits:?}"
        );
        let steps = wheel_steps();
        assert!(steps.down >= 0.0 && steps.across >= 0.0, "{steps:?}");
        let zero = ClickLimits {
            time: 0,
            width: 0,
            height: -1,
        };
        assert_eq!(
            usable(zero),
            ClickLimits {
                time: 1,
                width: 1,
                height: 1
            }
        );
    }
}
