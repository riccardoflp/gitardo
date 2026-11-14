//! Windows: what the libraries of the window do not read, asked of `user32.dll`, the folders watched and the
//! junctions made, asked of `kernel32.dll`, and the credential store, asked of `advapi32.dll`, from Windows 10 version
//! 1809.
//!
//! `user32.rs`, `kernel32.rs` and `advapi32.rs` declare what is used of each library, with the documentation's names;
//! `settings.rs`, `watch.rs`, `junction.rs` and `credentials.rs` wrap those calls in safe functions at once.

mod advapi32;
mod credentials;
mod junction;
mod kernel32;
mod settings;
mod shell;
mod shell32;
mod user32;
mod watch;

pub(crate) use credentials::{delete_secret, read_secret, write_secret};
pub(crate) use junction::make_junction;
pub(crate) use settings::{click_limits, is_high_contrast, utc_offset, wheel_steps};
pub(crate) use shell::open_url;
pub(crate) use watch::DirectoryWatch;
