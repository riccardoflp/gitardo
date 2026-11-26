//! macOS: the secrets, kept in the keychain of the user, and the folders watched, asked of `FSEvents`.
//!
//! `core_foundation.rs`, `core_services.rs`, `libsystem.rs` and `security.rs` declare what is used of each system
//! library, with the names Apple's documentation gives them; `objects.rs` owns the objects of `CoreFoundation`, and
//! `credentials.rs` and `watch.rs` wrap the calls in safe functions at once.

#[allow(
    missing_docs,
    reason = "the bindings are for platform, with the names of Apple's documentation, which says what each is"
)]
pub mod core_foundation;
mod core_services;
mod credentials;
mod libsystem;
pub mod objects;
mod security;
mod watch;

pub(crate) use credentials::{delete_secret, read_secret, write_secret};
pub(crate) use watch::DirectoryWatch;
