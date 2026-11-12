//! Gitardo Desktop: the windows, the event loop, the store, the settings and what is kept between runs.
//!
//! The library holds what the binaries share. For now it is the window that shows a view of the toolkit
//! ([`window::Surface`]): the input of the platform turned into the input of the toolkit, the fonts of the system, the
//! theme chosen or the system's, and the frame drawn, shown and drawn again when it asks to be.

#![forbid(unsafe_code)]

pub mod error;
pub mod window;

pub use error::AppError;
