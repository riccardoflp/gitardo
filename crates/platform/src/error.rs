//! What can go wrong when Gitardo Desktop asks something of the operating system.

use std::fmt;
use std::path::PathBuf;

use crate::geometry::PhysicalSize;

/// Why the operating system did not do what was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformError {
    /// A call into the system failed, with the code the system gave.
    System {
        /// The name of the call, as the system's documentation gives it.
        call: &'static str,
        /// The error code the system gave, or 0 when it gave none.
        code: u32,
    },
    /// The event loop or the window that a waker or an input sender points to is gone.
    Closed,
    /// A frame does not have as many pixels as its size says.
    FrameSize {
        /// How many pixels the frame has.
        pixels: usize,
        /// The size the frame says it has.
        size: PhysicalSize,
    },
    /// Another program kept the clipboard open.
    ClipboardBusy,
    /// A folder the system keeps something in could not be read.
    Folder {
        /// The folder.
        path: PathBuf,
        /// Why it could not be read.
        kind: std::io::ErrorKind,
    },
}

impl fmt::Display for PlatformError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlatformError::System { call, code } => {
                write!(formatter, "The system refused {call} (error {code}).")
            }
            PlatformError::Closed => write!(formatter, "The window or the event loop is already closed."),
            PlatformError::FrameSize { pixels, size } => write!(
                formatter,
                "A frame of {} by {} pixels came with {pixels} pixels.",
                size.width, size.height
            ),
            PlatformError::ClipboardBusy => {
                write!(
                    formatter,
                    "Another program is using the clipboard. Try again in a moment."
                )
            }
            PlatformError::Folder { path, kind } => {
                write!(formatter, "The folder {} could not be read: {kind}.", path.display())
            }
        }
    }
}

impl std::error::Error for PlatformError {}
