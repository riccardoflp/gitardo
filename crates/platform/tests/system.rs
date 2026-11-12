//! What the system says and keeps: the clipboard, the theme and the fonts.

// The helpers of these tests fail as the tests do, loudly: clippy allows them what it allows tests.
#![cfg(test)]
#![cfg(windows)]

use platform::{Clipboard, FontStyle, system_fonts, system_theme};

#[test]
fn round_trips_text_through_the_clipboard_and_puts_back_what_was_there() {
    let clipboard = Clipboard::new().unwrap();
    let before = clipboard.read_text().unwrap();
    if before.is_none() && !clipboard.is_empty() {
        // The clipboard holds something other than text, such as an image, which this test could not put back.
        return;
    }
    let text = "Gitardo Desktop: è, 😀, and a line\r\nbreak";
    clipboard.write_text(text).unwrap();
    let read = clipboard.read_text().unwrap();
    clipboard.write_text("").unwrap();
    let empty = clipboard.read_text().unwrap();
    match &before {
        Some(previous) => clipboard.write_text(previous).unwrap(),
        None => clipboard.write_text("").unwrap(),
    }
    assert_eq!(read.as_deref(), Some(text));
    assert_eq!(empty.as_deref(), Some(""));
    assert_eq!(clipboard.read_text().unwrap(), before.or(Some(String::new())));
}

#[test]
fn answers_what_the_theme_is() {
    let theme = system_theme().unwrap();
    assert_eq!(system_theme().unwrap(), theme);
}

#[test]
fn finds_the_fonts_of_the_interface_and_of_code() {
    let fonts = system_fonts().unwrap();
    assert_ne!(fonts.files.len(), 0);
    let interface = fonts.interface.unwrap();
    assert_eq!(interface.name, "Segoe UI");
    assert_eq!(interface.faces[0].style, FontStyle::Regular);
    assert!(interface.faces.iter().all(|face| face.path.is_file()));
    let monospace = fonts.monospace.unwrap();
    assert!(
        ["Cascadia Mono", "Consolas"].contains(&monospace.name.as_str()),
        "{}",
        monospace.name
    );
}
