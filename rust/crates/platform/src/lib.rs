//! The operating system, as the window of Gitardo Desktop sees it: windows, input, clipboard, timers, dialogs, opening
//! and revealing. What the engine asks of the system without a window, the credential store, the links to folders and
//! the folders watched, is in the crate `system`, whose bindings of the system libraries this crate uses too.
//!
//! The window, its input, the clipboard, the dialogs and the screen reader come from the libraries of decision D16
//! (`winit`, `softbuffer`, `arboard`, `rfd`, `accesskit_winit`), none of whose types appear here but the tree of
//! `accesskit`, re-exported as [`accesskit`] for the screen reader. What they do not cover is asked of the system,
//! and this crate and `system` are the ones allowed to use `unsafe` for it (rule R5.1).
//!
//! A program makes one [`EventLoop`] and calls [`EventLoop::run`] with a handler that gets every [`Event`] with a
//! [`Context`]: it opens its [`Window`]s on [`Event::Started`], and then gets the input of each window, the messages
//! other threads send through a [`Waker`], and the timers. It draws a frame of pixels and shows it with
//! [`Window::present`].

#![allow(unsafe_code)]
// On a system without a part of its own, what only the parts of Windows, Linux and macOS use is unused.
#![cfg_attr(
    not(any(windows, target_os = "linux", target_os = "macos")),
    allow(dead_code, unused_imports)
)]

mod accessibility;
mod click;
mod clipboard;
mod convert;
mod dialog;
mod error;
mod event;
mod event_loop;
mod frame;
mod geometry;
mod input;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod mailbox;
mod placement;
mod present;
mod shell;
mod system;
mod theme;
mod timers;
mod translate;
mod window;
mod window_state;
#[cfg(windows)]
mod windows;

/// The tree of what is on screen, as the screen reader takes it: `ui` builds it, a window hands it over.
pub use accesskit;

pub use accessibility::AccessibilityRequest;
pub use clipboard::Clipboard;
pub use dialog::{FileDialog, FileFilter};
pub use error::PlatformError;
pub use event::{Cursor, Event, TimerId, WindowId};
pub use event_loop::{Context, EventLoop};
pub use frame::{Decorations, FramePart, ResizeEdge};
pub use geometry::{LogicalSize, PhysicalSize, PixelRect, Point, Scale, ScreenPosition};
pub use input::{
    InputEvent, Key, KeyEvent, Modifiers, NamedKey, PointerButton, PointerButtonEvent, PointerMove, WheelDelta,
    WheelEvent,
};
pub use mailbox::{InputSender, Waker};
pub use placement::WindowPlacement;
pub use present::Frame;
pub use shell::open_url;
pub use system::utc_offset;
pub use theme::{Appearance, SystemTheme};
// The folders watched are the crate `system`'s, which the engine without a window watches with too (D93).
pub use ::system::{WatchDeliver, WatchEvent, Watcher};
pub use window::{TextInputArea, Visibility, Window, WindowOptions};
