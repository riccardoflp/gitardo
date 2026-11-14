//! The settings of the desktop that the libraries of the window do not read, asked of `gsettings` when the system has
//! it: high contrast, and how soon two clicks make a double click. A desktop without `gsettings` gets the usual values.

use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// How soon two presses make a double click when the desktop does not say, in milliseconds: GTK's own default.
const DOUBLE_CLICK_TIME: u32 = 400;

/// Whether high contrast is on, as GNOME and the desktops that read its settings keep it. Read again each time,
/// since the program asks only when a window gets the keyboard back; off when `gsettings` is not there.
pub(crate) fn is_high_contrast() -> bool {
    setting("org.gnome.desktop.a11y.interface", "high-contrast").as_deref() == Some("true")
}

/// How soon the next press must come to make a double click, in milliseconds: read once, since every press asks.
pub(crate) fn double_click_time() -> u32 {
    static TIME: OnceLock<u32> = OnceLock::new();
    *TIME.get_or_init(|| {
        setting("org.gnome.desktop.peripherals.mouse", "double-click")
            .and_then(|value| parse_integer(&value))
            .filter(|time| *time > 0)
            .unwrap_or(DOUBLE_CLICK_TIME)
    })
}

/// A setting as `gsettings get` prints it, without the line break; none when `gsettings` is not there or does not
/// know the setting.
fn setting(schema: &str, key: &str) -> Option<String> {
    let output = Command::new("gsettings")
        .args(["get", schema, key])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok().map(|text| text.trim().to_owned())
}

/// A whole number as `gsettings` prints it: `400`, or `int32 400` for a value whose type it names.
fn parse_integer(value: &str) -> Option<u32> {
    value.rsplit(' ').next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::{double_click_time, is_high_contrast, parse_integer};

    #[test]
    fn reads_the_numbers_gsettings_prints() {
        assert_eq!(parse_integer("400"), Some(400));
        assert_eq!(parse_integer("int32 250"), Some(250));
        assert_eq!(parse_integer("'four hundred'"), None);
        assert_eq!(parse_integer(""), None);
    }

    #[test]
    fn reads_the_settings_with_or_without_gsettings() {
        let _ = is_high_contrast();
        assert!(double_click_time() > 0);
    }
}
