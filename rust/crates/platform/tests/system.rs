//! What the system says and keeps: the clipboard.

// The helpers of these tests fail as the tests do, loudly: clippy allows them what it allows tests.
#![cfg(test)]
#![cfg(any(windows, target_os = "linux", target_os = "macos"))]

use platform::Clipboard;

#[test]
fn round_trips_text_through_the_clipboard_and_puts_back_what_was_there() {
    let has_display = |name: &str| std::env::var_os(name).is_some();
    if cfg!(target_os = "linux") && !has_display("DISPLAY") && !has_display("WAYLAND_DISPLAY") {
        // A Linux without a display server, such as a server, has no clipboard.
        return;
    }
    let clipboard = Clipboard::new();
    if cfg!(target_os = "linux") && !has_display("DISPLAY") && clipboard.is_err() {
        // A compositor of Wayland that offers neither ext-data-control nor wlr-data-control, as GNOME's and Weston's,
        // lets a program reach its clipboard only through XWayland, which is not there: the clipboard is refused,
        // as an error and not a crash.
        return;
    }
    let mut clipboard = clipboard.unwrap();
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
