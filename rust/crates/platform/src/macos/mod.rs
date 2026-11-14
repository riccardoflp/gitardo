//! macOS: what the libraries of the window do not read, asked of `AppKit` through the Objective-C runtime, the folders
//! watched, asked of `FSEvents`, addresses opened with `open`, and the local time, asked of the C library. The secrets
//! are kept in the keychain by the crate `system`.
//!
//! `core_services.rs`, `objc.rs` and `libsystem.rs` declare what is used of each system library, with the names Apple's
//! documentation gives them; those of `CoreFoundation`, and `objects.rs`, which owns its objects, are the crate
//! `system`'s. `settings.rs`, `watch.rs` and `clock.rs` wrap the calls in safe functions at once.

use ::system::macos::{core_foundation, objects};

mod clock;
mod core_services;
mod libsystem;
mod objc;
mod settings;
mod watch;

pub(crate) use clock::utc_offset;
pub(crate) use settings::{double_click_time, is_high_contrast};
pub(crate) use watch::DirectoryWatch;
