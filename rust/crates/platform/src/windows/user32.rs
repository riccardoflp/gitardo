//! The functions, types and constants of `user32.dll` that Gitardo Desktop asks for itself, with the names the Windows
//! documentation gives them: the settings of the system that the libraries of the window do not read.

#![allow(
    clippy::upper_case_acronyms,
    reason = "the names are those of the Windows documentation"
)]
#![allow(
    non_snake_case,
    reason = "the names of the fields are those of the Windows documentation"
)]

use core::ffi::c_void;

/// A boolean of the system: 0 is false, anything else is true.
pub(crate) type BOOL = i32;

/// The high contrast setting.
#[repr(C)]
pub(crate) struct HIGHCONTRASTW {
    pub(crate) cbSize: u32,
    pub(crate) dwFlags: u32,
    pub(crate) lpszDefaultScheme: *mut u16,
}

#[link(name = "user32")]
unsafe extern "system" {
    pub(crate) fn GetDoubleClickTime() -> u32;
    pub(crate) fn GetSystemMetricsForDpi(index: i32, dpi: u32) -> i32;
    pub(crate) fn SystemParametersInfoW(action: u32, parameter: u32, value: *mut c_void, update: u32) -> BOOL;
}

pub(crate) const SM_CXDOUBLECLK: i32 = 36;
pub(crate) const SM_CYDOUBLECLK: i32 = 37;
pub(crate) const SPI_GETWHEELSCROLLLINES: u32 = 0x0068;
pub(crate) const SPI_GETWHEELSCROLLCHARS: u32 = 0x006C;
pub(crate) const SPI_GETHIGHCONTRAST: u32 = 0x0042;
pub(crate) const HCF_HIGHCONTRASTON: u32 = 0x0000_0001;
/// What the wheel settings say when a notch should scroll a whole page.
pub(crate) const WHEEL_PAGESCROLL: u32 = u32::MAX;
