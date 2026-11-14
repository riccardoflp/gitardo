//! The settings of Windows that the libraries of the window do not read: high contrast, how near and how soon two
//! clicks make a double click, how far a notch of the wheel scrolls, and how far the local time is from UTC.

use core::ffi::c_void;

use super::{kernel32, user32};
use crate::click::ClickLimits;
use crate::geometry::Scale;
use crate::system::{LINES_FOR_A_PAGE, WheelSteps};

/// Whether high contrast is on. A failure, which only a wrong size of the setting would cause, reads as off.
pub(crate) fn is_high_contrast() -> bool {
    let size = u32::try_from(size_of::<user32::HIGHCONTRASTW>()).unwrap_or_default();
    let mut setting = user32::HIGHCONTRASTW {
        cbSize: size,
        dwFlags: 0,
        lpszDefaultScheme: std::ptr::null_mut(),
    };
    let pointer = (&raw mut setting).cast::<c_void>();
    // SAFETY: `setting` is a HIGHCONTRASTW with its size filled in, as SPI_GETHIGHCONTRAST asks, alive for the call.
    // The name of the scheme it points to afterwards belongs to the system and is not read.
    let result = unsafe { user32::SystemParametersInfoW(user32::SPI_GETHIGHCONTRAST, size, pointer, 0) };
    result != 0 && setting.dwFlags & user32::HCF_HIGHCONTRASTON != 0
}

/// How soon and how near the next press must come to make a double click, at the scale of a window.
pub(crate) fn click_limits(scale: Scale) -> ClickLimits {
    // SAFETY: the call takes nothing.
    let time = unsafe { user32::GetDoubleClickTime() };
    // SAFETY: only integers are passed; an index the system does not know gives 0.
    let width = unsafe { user32::GetSystemMetricsForDpi(user32::SM_CXDOUBLECLK, scale.dpi()) };
    // SAFETY: as above.
    let height = unsafe { user32::GetSystemMetricsForDpi(user32::SM_CYDOUBLECLK, scale.dpi()) };
    ClickLimits { time, width, height }
}

/// How many lines a notch of the wheel scrolls down, and how many characters across, as the person set them.
pub(crate) fn wheel_steps() -> WheelSteps {
    WheelSteps {
        down: wheel_setting(user32::SPI_GETWHEELSCROLLLINES),
        across: wheel_setting(user32::SPI_GETWHEELSCROLLCHARS),
    }
}

/// How far the local time is from UTC now, in seconds, with daylight saving time as it is now. A failure reads as UTC.
pub(crate) fn utc_offset() -> i64 {
    let mut now = kernel32::FILETIME::default();
    // SAFETY: `now` is a FILETIME alive for the call, which only writes it.
    unsafe { kernel32::GetSystemTimeAsFileTime(&raw mut now) };
    let mut local = kernel32::FILETIME::default();
    // SAFETY: `now` is a FILETIME the system filled in, `local` one alive for the call, which only writes it.
    let result = unsafe { kernel32::FileTimeToLocalFileTime(&raw const now, &raw mut local) };
    if result == 0 {
        return 0;
    }
    let intervals = |time: kernel32::FILETIME| (i64::from(time.dwHighDateTime) << 32) | i64::from(time.dwLowDateTime);
    // A FILETIME counts 100-nanosecond intervals: ten million make a second.
    (intervals(local) - intervals(now)) / 10_000_000
}

/// One setting of the wheel, in lines or characters; a page is [`LINES_FOR_A_PAGE`], and a failure the usual 3.
fn wheel_setting(action: u32) -> f32 {
    let mut value: u32 = 3;
    let pointer = (&raw mut value).cast::<c_void>();
    // SAFETY: both settings write a u32 into `value`, which is alive for the call.
    let result = unsafe { user32::SystemParametersInfoW(action, 0, pointer, 0) };
    match (result, value) {
        (0, _) => 3.0,
        (_, user32::WHEEL_PAGESCROLL) => LINES_FOR_A_PAGE,
        (_, steps) => f32::from(u16::try_from(steps).unwrap_or(u16::MAX)),
    }
}
