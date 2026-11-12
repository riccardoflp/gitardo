//! The base types of the Windows API that every library uses (windef.h, minwindef.h, winnt.h), with the names the
//! documentation gives them.
//!
//! Handles are kept as integers of the size of a pointer: the system gives and takes them as opaque values, which
//! makes them plain data that can cross threads as the documentation allows for `PostMessageW`.

#![allow(
    clippy::upper_case_acronyms,
    reason = "the names are those of the Windows documentation"
)]

/// A boolean of the system: 0 is false, anything else is true.
pub(crate) type BOOL = i32;
/// A handle to a window.
pub(crate) type HWND = isize;
/// A handle to a module, such as the program itself.
pub(crate) type HINSTANCE = isize;
/// A handle to a device context, something GDI draws into.
pub(crate) type HDC = isize;
/// A handle to a cursor.
pub(crate) type HCURSOR = isize;
/// A handle to an icon.
pub(crate) type HICON = isize;
/// A handle to a brush.
pub(crate) type HBRUSH = isize;
/// A handle to a menu.
pub(crate) type HMENU = isize;
/// A handle to an object of any kind, such as the data of the clipboard.
pub(crate) type HANDLE = isize;
/// A handle to memory from `GlobalAlloc`.
pub(crate) type HGLOBAL = isize;
/// The first parameter of a message.
pub(crate) type WPARAM = usize;
/// The second parameter of a message.
pub(crate) type LPARAM = isize;
/// What a window procedure returns.
pub(crate) type LRESULT = isize;

/// The value of a `BOOL` that is false.
pub(crate) const FALSE: BOOL = 0;

/// A rectangle by its edges.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RECT {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
}

/// A point.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct POINT {
    pub(crate) x: i32,
    pub(crate) y: i32,
}
