//! The clipboard of the system, for text, through `arboard`.

use crate::error::PlatformError;

/// The clipboard. On Linux, text written stays on the clipboard while this is kept, so a program keeps one.
pub struct Clipboard {
    library: arboard::Clipboard,
}

impl Clipboard {
    /// The clipboard of the system.
    ///
    /// # Errors
    ///
    /// `PlatformError::System` when the system has no clipboard the program can reach.
    pub fn new() -> Result<Clipboard, PlatformError> {
        let library = arboard::Clipboard::new().map_err(clipboard_error)?;
        Ok(Clipboard { library })
    }

    /// The text on the clipboard; none when it holds no text, such as an image, or nothing.
    ///
    /// # Errors
    ///
    /// `PlatformError::ClipboardBusy` when another program kept it open; `PlatformError::System` when it could not
    /// be read.
    pub fn read_text(&mut self) -> Result<Option<String>, PlatformError> {
        match self.library.get_text() {
            Ok(text) => Ok(Some(text)),
            Err(arboard::Error::ContentNotAvailable) => Ok(None),
            Err(error) => Err(clipboard_error(error)),
        }
    }

    /// Puts text on the clipboard, in place of what was there.
    ///
    /// # Errors
    ///
    /// `PlatformError::ClipboardBusy` when another program kept it open; `PlatformError::System` when it could not
    /// be written.
    pub fn write_text(&mut self, text: &str) -> Result<(), PlatformError> {
        self.library.set_text(text).map_err(clipboard_error)
    }
}

/// The error of the clipboard.
fn clipboard_error(error: arboard::Error) -> PlatformError {
    match error {
        arboard::Error::ClipboardOccupied => PlatformError::ClipboardBusy,
        other => PlatformError::system("arboard, to use the clipboard", &other),
    }
}
