//! Windows: what the libraries of the window do not read, asked of `user32.dll`, and the folders watched, asked of
//! `kernel32.dll`, from Windows 10 version 1809.
//!
//! `user32.rs` and `kernel32.rs` declare what is used of each library, with the documentation's names; `settings.rs`
//! and `watch.rs` wrap those calls in safe functions at once.

mod kernel32;
mod settings;
mod user32;
mod watch;

pub(crate) use settings::{click_limits, is_high_contrast, wheel_steps};
pub(crate) use watch::{DirectoryWatch, Waited};
