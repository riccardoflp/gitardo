//! Windows: what the libraries of the window do not read, asked of `user32.dll`, from Windows 10 version 1809.
//!
//! `user32.rs` declares what is used of the library, with the documentation's names; `settings.rs` wraps those calls
//! in safe functions at once.

mod settings;
mod user32;

pub(crate) use settings::{click_limits, is_high_contrast, wheel_steps};
