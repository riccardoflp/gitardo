//! Windows: what the libraries of the window do not read, asked of `user32.dll`, and the folders watched and the local
//! time, asked of `kernel32.dll`, from Windows 10 version 1809.
//!
//! `frame.rs` answers what the system asks of a window that draws its own title bar, with the procedure of
//! `comctl32.dll` it puts in front of the window's.
//!
//! `user32.rs`, `shell32.rs` and `comctl32.rs` declare what is used of each library, with the documentation's names; the declarations
//! of `kernel32.dll` are those of the crate `system`, which makes the junctions with them. `settings.rs`, `watch.rs`
//! and `shell.rs` wrap those calls in safe functions at once.

use ::system::windows::kernel32;

mod comctl32;
mod frame;
mod settings;
mod shell;
mod shell32;
mod user32;
mod watch;

pub(crate) use frame::FrameProcedure;
pub(crate) use settings::{click_limits, is_high_contrast, utc_offset, wheel_steps};
pub(crate) use shell::open_url;
pub(crate) use watch::DirectoryWatch;
