//! Windows: what the libraries of the window do not read, asked of `user32.dll`, and the folders watched and the
//! junctions made, asked of `kernel32.dll`, from Windows 10 version 1809.
//!
//! `user32.rs` and `kernel32.rs` declare what is used of each library, with the documentation's names; `settings.rs`,
//! `watch.rs` and `junction.rs` wrap those calls in safe functions at once.

mod junction;
mod kernel32;
mod settings;
mod user32;
mod watch;

pub(crate) use junction::make_junction;
pub(crate) use settings::{click_limits, is_high_contrast, wheel_steps};
pub(crate) use watch::{DirectoryWatch, Waited};
