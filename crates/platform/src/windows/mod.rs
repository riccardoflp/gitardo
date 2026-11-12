//! Windows: Win32 through the libraries every Windows has, from Windows 10 version 1809.
//!
//! The files named after a library (`user32.rs`, `gdi32.rs`…) declare what is used of it, with the documentation's
//! names; every other file wraps those calls in safe functions at once.

mod advapi32;
mod clipboard;
mod cursor;
mod dwmapi;
mod event_loop;
mod fonts;
mod gdi32;
mod handle;
mod ime;
mod imm32;
mod kernel32;
mod keyboard;
mod keys;
mod mailbox;
mod pointer;
mod present;
mod procedure;
mod theme;
mod types;
mod user32;
mod window;

pub use clipboard::Clipboard;
pub use event_loop::EventLoop;
pub use fonts::system_fonts;
pub use mailbox::{InputSender, Waker};
pub use present::Frame;
pub use theme::system_theme;
pub use window::{Visibility, Window, WindowOptions};

use crate::error::PlatformError;

/// A string as the system takes it: UTF-16, ended by a 0.
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// The error of a call that just failed, with the code the system kept for this thread.
fn last_error(call: &'static str) -> PlatformError {
    // SAFETY: GetLastError takes nothing and only reads a value of the calling thread.
    let code = unsafe { kernel32::GetLastError() };
    PlatformError::System { call, code }
}

/// A call that returns a `BOOL`, as a `Result`.
fn check(call: &'static str, result: types::BOOL) -> Result<(), PlatformError> {
    if result == types::FALSE {
        Err(last_error(call))
    } else {
        Ok(())
    }
}

/// The low 16 bits of a parameter of a message.
fn low_word(value: usize) -> u16 {
    u16::try_from(value & 0xFFFF).unwrap_or_default()
}

/// The 16 bits above the low 16 bits of a parameter of a message.
fn high_word(value: usize) -> u16 {
    low_word(value >> 16)
}

/// The two signed 16-bit coordinates packed in the `LPARAM` of a pointer message.
fn coordinates(lparam: types::LPARAM) -> (i16, i16) {
    let bits = lparam.cast_unsigned();
    (low_word(bits).cast_signed(), high_word(bits).cast_signed())
}

#[cfg(test)]
mod tests {
    use super::{coordinates, high_word, low_word, wide};

    #[test]
    fn unpacks_the_parameters_of_messages() {
        assert_eq!(low_word(0x1234_5678), 0x5678);
        assert_eq!(high_word(0x1234_5678), 0x1234);
        assert_eq!(coordinates(0x0014_000A), (10, 20));
        assert_eq!(coordinates(0xFFFE_FFFF), (-1, -2));
        assert_eq!(coordinates(-1), (-1, -1));
        assert_eq!(wide("ab"), vec![0x61, 0x62, 0]);
    }
}
