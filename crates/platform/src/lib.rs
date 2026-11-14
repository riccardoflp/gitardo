//! The operating system, as Gitardo Desktop sees it: windows, input, clipboard, timers, file watching, dialogs, credentials, opening and revealing.
//!
//! The window, its input, the clipboard, the dialogs and the screen reader come from the libraries of decision D16
//! (`winit`, `softbuffer`, `arboard`, `rfd`, `accesskit_winit`), none of whose types appear here but the tree of
//! `accesskit`, re-exported as [`accesskit`] for the screen reader. What they do not cover is asked of the system,
//! and this is the one crate allowed to use `unsafe` for it (rule R5.1).
//!
//! A program makes one [`EventLoop`] and calls [`EventLoop::run`] with a handler that gets every [`Event`] with a
//! [`Context`]: it opens its [`Window`]s on [`Event::Started`], and then gets the input of each window, the messages
//! other threads send through a [`Waker`], and the timers. It draws a frame of pixels and shows it with
//! [`Window::present`].

#![allow(unsafe_code)]
// Until macOS has its part, what only the parts for Windows and Linux use is unused there.
#![cfg_attr(not(any(windows, target_os = "linux")), allow(dead_code, unused_imports))]

mod accessibility;
mod click;
mod clipboard;
mod convert;
mod credentials;
mod dialog;
mod error;
mod event;
mod event_loop;
mod geometry;
mod input;
mod links;
#[cfg(target_os = "linux")]
mod linux;
mod mailbox;
mod present;
mod shell;
mod system;
mod theme;
mod timers;
mod translate;
mod watch;
mod window;
mod window_state;
#[cfg(windows)]
mod windows;

/// The tree of what is on screen, as the screen reader takes it: `ui` builds it, a window hands it over.
pub use accesskit;

pub use accessibility::AccessibilityRequest;
pub use clipboard::Clipboard;
pub use credentials::{delete_secret, read_secret, write_secret};
pub use dialog::{FileDialog, FileFilter};
pub use error::PlatformError;
pub use event::{Cursor, Event, TimerId, WindowId};
pub use event_loop::{Context, EventLoop};
pub use geometry::{LogicalSize, PhysicalSize, PixelRect, Point, Scale, ScreenPosition};
pub use input::{
    InputEvent, Key, KeyEvent, Modifiers, NamedKey, PointerButton, PointerButtonEvent, PointerMove, WheelDelta,
    WheelEvent,
};
pub use links::link_folder;
pub use mailbox::{InputSender, Waker};
pub use present::Frame;
pub use shell::open_url;
pub use system::utc_offset;
pub use theme::{Appearance, SystemTheme};
pub use watch::{WatchDeliver, WatchEvent, Watcher};
pub use window::{TextInputArea, Visibility, Window, WindowOptions};
