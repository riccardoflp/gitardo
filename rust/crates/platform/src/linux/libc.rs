//! The functions, types and constants of the C library that Gitardo Desktop asks for itself, with the names its manual
//! pages give them: the local time. The C library is linked by the standard library already; nothing more is linked.
//!
//! The values of the flags are those of `x86_64`, `aarch64` and `riscv64`, the systems Gitardo is built for.

#![allow(
    non_camel_case_types,
    reason = "the names of the types are those of the manual pages"
)]

use core::ffi::{c_char, c_int, c_long};

/// A moment, in seconds since 1970 in UTC.
pub(crate) type time_t = i64;

/// A moment broken into the fields of the local time, as `localtime_r` fills it. Gitardo reads only `tm_gmtoff`.
#[repr(C)]
#[allow(
    dead_code,
    reason = "the C library fills every field, and Gitardo reads only the offset"
)]
#[allow(
    clippy::struct_field_names,
    reason = "the names of the fields are those of the manual pages"
)]
pub(crate) struct tm {
    pub(crate) tm_sec: c_int,
    pub(crate) tm_min: c_int,
    pub(crate) tm_hour: c_int,
    pub(crate) tm_mday: c_int,
    pub(crate) tm_mon: c_int,
    pub(crate) tm_year: c_int,
    pub(crate) tm_wday: c_int,
    pub(crate) tm_yday: c_int,
    pub(crate) tm_isdst: c_int,
    /// How far the local time is from UTC, in seconds east of it.
    pub(crate) tm_gmtoff: c_long,
    pub(crate) tm_zone: *const c_char,
}

unsafe extern "C" {
    pub(crate) fn localtime_r(timep: *const time_t, result: *mut tm) -> *mut tm;
}
