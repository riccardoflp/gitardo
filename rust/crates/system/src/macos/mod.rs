//! macOS: the secrets, kept in the keychain of the user.
//!
//! `core_foundation.rs` and `security.rs` declare what is used of each system library, with the names Apple's
//! documentation gives them; `objects.rs` owns the objects of `CoreFoundation`, and `credentials.rs` wraps the calls in
//! safe functions at once. `platform` uses `core_foundation` and `objects` too, for the folders it watches.

#[allow(
    missing_docs,
    reason = "the bindings are for platform, with the names of Apple's documentation, which says what each is"
)]
pub mod core_foundation;
mod credentials;
pub mod objects;
mod security;

pub(crate) use credentials::{delete_secret, read_secret, write_secret};
