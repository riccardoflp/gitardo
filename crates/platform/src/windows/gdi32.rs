//! The functions, types and constants of `gdi32.dll` that Gitardo Desktop uses: copying a frame of pixels into a
//! window, with the names the Windows documentation gives them.

#![allow(
    clippy::upper_case_acronyms,
    reason = "the names are those of the Windows documentation"
)]
#![allow(
    non_snake_case,
    reason = "the names of the fields are those of the Windows documentation"
)]

use core::ffi::c_void;

use super::types::HDC;

/// What a device-independent bitmap is: its size, and how its pixels are laid out.
#[repr(C)]
pub(crate) struct BITMAPINFOHEADER {
    pub(crate) biSize: u32,
    pub(crate) biWidth: i32,
    pub(crate) biHeight: i32,
    pub(crate) biPlanes: u16,
    pub(crate) biBitCount: u16,
    pub(crate) biCompression: u32,
    pub(crate) biSizeImage: u32,
    pub(crate) biXPelsPerMeter: i32,
    pub(crate) biYPelsPerMeter: i32,
    pub(crate) biClrUsed: u32,
    pub(crate) biClrImportant: u32,
}

/// A color of the table of a bitmap; a bitmap of 32 bits per pixel has none, but the structure has room for one.
#[repr(C)]
pub(crate) struct RGBQUAD {
    pub(crate) rgbBlue: u8,
    pub(crate) rgbGreen: u8,
    pub(crate) rgbRed: u8,
    pub(crate) rgbReserved: u8,
}

/// A bitmap's header and its color table.
#[repr(C)]
pub(crate) struct BITMAPINFO {
    pub(crate) bmiHeader: BITMAPINFOHEADER,
    pub(crate) bmiColors: [RGBQUAD; 1],
}

#[link(name = "gdi32")]
unsafe extern "system" {
    pub(crate) fn StretchDIBits(
        context: HDC,
        destination_x: i32,
        destination_y: i32,
        destination_width: i32,
        destination_height: i32,
        source_x: i32,
        source_y: i32,
        source_width: i32,
        source_height: i32,
        bits: *const c_void,
        info: *const BITMAPINFO,
        usage: u32,
        operation: u32,
    ) -> i32;
}

pub(crate) const BI_RGB: u32 = 0;
pub(crate) const DIB_RGB_COLORS: u32 = 0;
pub(crate) const SRCCOPY: u32 = 0x00CC_0020;
