//! The operating system, as Gitardo Desktop sees it: windows, input, clipboard, timers, file watching, dialogs, credentials, opening and revealing.
//!
//! This is the one crate allowed to use `unsafe` (rule R5.1): every call into the system is wrapped here in a safe
//! function, so that the crates above see none of it.
//!
//! A program makes one [`EventLoop`] per UI thread, opens its [`Window`]s on it, and calls [`EventLoop::run`] with a
//! handler that gets every [`Event`]: the input of each window, the messages other threads send through a [`Waker`],
//! and the timers. It draws a frame of pixels and shows it with [`Window::present`].

#![allow(unsafe_code)]
// Until the other systems have their part, what only the part for Windows uses is unused elsewhere.
#![cfg_attr(not(windows), allow(dead_code, unused_imports))]

mod click;
mod convert;
mod dispatch;
mod error;
mod event;
mod fonts;
mod geometry;
mod input;
mod theme;
#[cfg(windows)]
mod windows;

pub use error::PlatformError;
pub use event::{Cursor, Event, TimerId, WindowId};
pub use fonts::{FontFace, FontFamily, FontStyle, SystemFonts};
pub use geometry::{LogicalSize, PhysicalSize, PixelRect, Point, Scale, ScreenPosition};
pub use input::{
    InputEvent, Key, KeyEvent, Modifiers, NamedKey, PointerButton, PointerButtonEvent, PointerMove, WheelDelta,
    WheelEvent,
};
pub use theme::{Appearance, SystemTheme};
#[cfg(windows)]
pub use windows::{
    Clipboard, EventLoop, Frame, InputSender, Visibility, Waker, Window, WindowOptions, system_fonts, system_theme,
};
