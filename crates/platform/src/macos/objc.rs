//! The functions of the Objective-C runtime, `libobjc`, that Gitardo Desktop asks for itself, with the names Apple's
//! documentation gives them: finding a class and a selector, and sending a message. `AppKit`, whose classes are asked,
//! is linked here too; `winit` links it as well.

#![allow(
    non_camel_case_types,
    clippy::upper_case_acronyms,
    reason = "the names of the types are those of Apple's documentation"
)]

use core::ffi::{c_char, c_void};

/// An object, or a class, of Objective-C.
pub(crate) type id = *mut c_void;
/// The name of a method, as the runtime keeps it.
pub(crate) type SEL = *const c_void;

#[link(name = "objc")]
unsafe extern "C" {
    pub(crate) fn objc_getClass(name: *const c_char) -> id;
    pub(crate) fn sel_registerName(name: *const c_char) -> SEL;
    /// Sends a message: declared without its arguments and its result, which change with the method, so that each
    /// call casts it to the signature of the method it sends.
    pub(crate) fn objc_msgSend();
}

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}
