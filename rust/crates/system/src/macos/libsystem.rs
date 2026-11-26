//! The functions and types of `libSystem` that Gitardo asks for itself on macOS, with the names its manual pages give
//! them: the queues of `libdispatch` the streams of `FSEvents` call on. `libSystem` is linked by the standard library
//! already; nothing more is linked.

#![allow(
    non_camel_case_types,
    reason = "the names of the types are those of the manual pages"
)]

use core::ffi::{c_char, c_void};

/// A queue of `libdispatch`, on which work runs one piece after the other.
pub(crate) type dispatch_queue_t = *mut c_void;

unsafe extern "C" {
    /// A queue: a null attribute makes it run one piece of work at a time.
    pub(crate) fn dispatch_queue_create(label: *const c_char, attributes: *const c_void) -> dispatch_queue_t;
    pub(crate) fn dispatch_release(object: *mut c_void);
}
