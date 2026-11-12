//! What can keep a window of Gitardo Desktop from opening or from drawing.

use std::fmt;

use platform::PlatformError;

/// Why a window could not open, draw or show what it drew.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    /// The operating system refused something.
    Platform(PlatformError),
    /// A typeface of the interface could not be made of the fonts loaded, as the renderer says.
    Font(render::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Platform(error) => write!(formatter, "{error}"),
            AppError::Font(error) => write!(formatter, "The fonts of the interface could not be loaded: {error}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<PlatformError> for AppError {
    fn from(error: PlatformError) -> AppError {
        AppError::Platform(error)
    }
}
