//! macOS: what the libraries of the window do not read, asked of `AppKit` through the Objective-C runtime, the folders
//! watched, asked of `FSEvents`, the secrets, kept in the keychain of the user, addresses opened with `open`, and the
//! local time, asked of the C library.
//!
//! `core_foundation.rs`, `core_services.rs`, `security.rs`, `objc.rs` and `libsystem.rs` declare what is used of each
//! system library, with the names Apple's documentation gives them; `objects.rs` owns the objects of `CoreFoundation`,
//! and `settings.rs`, `watch.rs`, `credentials.rs` and `clock.rs` wrap the calls in safe functions at once.

mod clock;
mod core_foundation;
mod core_services;
mod credentials;
mod libsystem;
mod objc;
mod objects;
mod security;
mod settings;
mod watch;

pub(crate) use clock::utc_offset;
pub(crate) use credentials::{delete_secret, read_secret, write_secret};
pub(crate) use settings::{double_click_time, is_high_contrast};
pub(crate) use watch::DirectoryWatch;
