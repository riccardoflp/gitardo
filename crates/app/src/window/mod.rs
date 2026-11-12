//! A window that shows a view of the toolkit: what turns the events of the platform into frames of the toolkit and
//! the frames back into pixels on the screen.
//!
//! A frame is drawn only when the window asks for one: after input, when a timer the last frame asked for goes off,
//! when the theme or the size changes. Only the parts of the frame that changed are sent to the screen.

pub mod fonts;
pub mod input;
pub mod theme;

#[cfg(windows)]
mod surface;

#[cfg(windows)]
pub use surface::{Step, Surface, SurfaceOptions};
