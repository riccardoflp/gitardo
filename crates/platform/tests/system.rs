//! What the system says and keeps: the clipboard.

// The helpers of these tests fail as the tests do, loudly: clippy allows them what it allows tests.
#![cfg(test)]
#![cfg(any(windows, target_os = "linux"))]

use platform::Clipboard;

#[test]
fn round_trips_text_through_the_clipboard_and_puts_back_what_was_there() {
    if cfg!(target_os = "linux") && std::env::var_os("DISPLAY").is_none() {
        // The clipboard goes through X11, or XWayland on Wayland: without a display there is none.
        return;
    }
    let mut clipboard = Clipboard::new().unwrap();
    let Some(before) = clipboard.read_text().unwrap() else {
        // The clipboard holds no text: it may hold an image, which this test could not put back, so it is left alone.
        return;
    };
    let text = "Gitardo Desktop: è, 😀, and a line\r\nbreak";
    clipboard.write_text(text).unwrap();
    let read = clipboard.read_text().unwrap();
    clipboard.write_text(&before).unwrap();
    assert_eq!(read.as_deref(), Some(text));
    assert_eq!(clipboard.read_text().unwrap(), Some(before));
}
