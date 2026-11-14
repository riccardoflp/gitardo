//! What can go wrong when Gitardo Desktop asks something of the operating system.

use std::fmt;

use crate::geometry::PhysicalSize;

/// Why the operating system did not do what was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformError {
    /// The system, or the library that speaks to it, refused, and said why in its own words.
    System {
        /// What was asked: the call, or the library and what it was asked.
        call: &'static str,
        /// What the system or the library said, kept to be shown under the details of the error.
        details: String,
    },
    /// The program already has its event loop: a program has one, for the life of the program.
    EventLoopExists,
    /// The system cannot do what was asked, such as saying where a window is on Wayland.
    Unsupported {
        /// What was asked.
        what: &'static str,
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
}

impl PlatformError {
    /// The error of a library that refused, with its words.
    pub(crate) fn system(call: &'static str, error: &impl fmt::Display) -> PlatformError {
        PlatformError::System {
            call,
            details: error.to_string(),
        }
    }
}

impl From<::system::SystemError> for PlatformError {
    fn from(error: ::system::SystemError) -> PlatformError {
        match error {
            ::system::SystemError::System { call, details } => PlatformError::System { call, details },
            ::system::SystemError::Unsupported { what } => PlatformError::Unsupported { what },
        }
    }
}

impl fmt::Display for PlatformError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlatformError::System { call, details } => write!(formatter, "The system refused {call}: {details}"),
            PlatformError::EventLoopExists => write!(formatter, "The program already has its event loop."),
            PlatformError::Unsupported { what } => write!(formatter, "This system cannot {what}."),
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
        }
    }
}

impl std::error::Error for PlatformError {}
