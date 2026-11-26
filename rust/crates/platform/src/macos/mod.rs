//! macOS: what the libraries of the window do not read, asked of `AppKit` through the Objective-C runtime, addresses
//! opened with `open`, and the local time, asked of the C library. The secrets are kept in the keychain, and the
//! folders watched with `FSEvents`, by the crate `system`.
//!
//! `objc.rs` and `libsystem.rs` declare what is used of each system library, with the names Apple's documentation
//! gives them. `settings.rs` and `clock.rs` wrap the calls in safe functions at once.

mod clock;
mod libsystem;
mod objc;
mod settings;

pub(crate) use clock::utc_offset;
pub(crate) use settings::{double_click_time, is_high_contrast};
