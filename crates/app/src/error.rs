//! What can keep a window of Gitardo Desktop from opening or from drawing.

use std::fmt;
use std::path::PathBuf;

use platform::PlatformError;

/// Why a window could not open, draw or show what it drew.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    /// The operating system refused something.
    Platform(PlatformError),
    /// The system has none of the fonts the interface is drawn with.
    NoInterfaceFont,
    /// A font file the interface needs could not be read.
    FontFile {
        /// The file.
        path: PathBuf,
        /// Why it could not be read from the disk.
        kind: std::io::ErrorKind,
    },
    /// A font file the interface needs is not a font the renderer can draw.
    Font {
        /// The file.
        path: PathBuf,
        /// What the renderer found wrong with it.
        error: render::Error,
    },
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Platform(error) => write!(formatter, "{error}"),
            AppError::NoInterfaceFont => write!(
                formatter,
                "The system has no font for the interface: Segoe UI is missing."
            ),
            AppError::FontFile { path, kind } => {
                write!(formatter, "The font file {} could not be read: {kind}.", path.display())
            }
            AppError::Font { path, error } => write!(formatter, "{}: {error}", path.display()),
        }
    }
}

impl std::error::Error for AppError {}

impl From<PlatformError> for AppError {
    fn from(error: PlatformError) -> AppError {
        AppError::Platform(error)
    }
}
