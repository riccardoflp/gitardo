//! The settings of the system that the libraries of the window do not read, asked of `AppKit` through the Objective-C
//! runtime: high contrast, and how soon two clicks make a double click.

use core::ffi::CStr;
use std::sync::OnceLock;

use super::objc::{self, SEL, id};

/// How soon two presses make a double click when the system does not say, in milliseconds: the usual half second.
const DOUBLE_CLICK_TIME: u32 = 500;

/// A message to an object that gives back an object.
type SendsObject = unsafe extern "C" fn(receiver: id, selector: SEL) -> id;
/// A message to an object that gives back a `BOOL`, one byte on every Mac.
type SendsBool = unsafe extern "C" fn(receiver: id, selector: SEL) -> u8;
/// A message to an object that gives back a length of time, in seconds.
type SendsSeconds = unsafe extern "C" fn(receiver: id, selector: SEL) -> f64;

/// A class of `AppKit` and a selector, both found by name; none when the runtime has no such class.
fn class_and_selector(class: &CStr, selector: &CStr) -> Option<(id, SEL)> {
    // SAFETY: both names are C strings alive for the calls; the runtime only reads them.
    let (class, selector) = unsafe {
        (
            objc::objc_getClass(class.as_ptr()),
            objc::sel_registerName(selector.as_ptr()),
        )
    };
    (!class.is_null() && !selector.is_null()).then_some((class, selector))
}

/// Whether the user asked the system for more contrast (Accessibility, Display, Increase contrast). Read again each
/// time, since the program asks only when a window gets the keyboard back.
pub(crate) fn is_high_contrast() -> bool {
    let Some((class, shared)) = class_and_selector(c"NSWorkspace", c"sharedWorkspace") else {
        return false;
    };
    // SAFETY: `objc_msgSend` is called with the signature of each method it sends: `+sharedWorkspace` gives an object,
    // which lives as long as the program, and `-accessibilityDisplayShouldIncreaseContrast` gives a `BOOL`.
    unsafe {
        let sends_object = core::mem::transmute::<unsafe extern "C" fn(), SendsObject>(objc::objc_msgSend);
        let sends_bool = core::mem::transmute::<unsafe extern "C" fn(), SendsBool>(objc::objc_msgSend);
        let workspace = sends_object(class, shared);
        let asked = objc::sel_registerName(c"accessibilityDisplayShouldIncreaseContrast".as_ptr());
        !workspace.is_null() && !asked.is_null() && sends_bool(workspace, asked) != 0
    }
}

/// How soon the next press must come to make a double click, in milliseconds, as the user set it: read once, since
/// every press asks.
pub(crate) fn double_click_time() -> u32 {
    static TIME: OnceLock<u32> = OnceLock::new();
    *TIME.get_or_init(|| {
        let Some((class, interval)) = class_and_selector(c"NSEvent", c"doubleClickInterval") else {
            return DOUBLE_CLICK_TIME;
        };
        // SAFETY: `objc_msgSend` is called with the signature of `+doubleClickInterval`, which gives a length of time.
        let seconds = unsafe {
            let sends_seconds = core::mem::transmute::<unsafe extern "C" fn(), SendsSeconds>(objc::objc_msgSend);
            sends_seconds(class, interval)
        };
        milliseconds(seconds).unwrap_or(DOUBLE_CLICK_TIME)
    })
}

/// A length of time in seconds as whole milliseconds; none when it is not one a double click can have.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is checked to be between 1 and 5000 milliseconds before the cast"
)]
fn milliseconds(seconds: f64) -> Option<u32> {
    let milliseconds = (seconds * 1000.0).round();
    (1.0..=5000.0).contains(&milliseconds).then_some(milliseconds as u32)
}

#[cfg(test)]
mod tests {
    use super::{double_click_time, is_high_contrast, milliseconds};

    #[test]
    fn turns_seconds_into_the_milliseconds_of_a_double_click() {
        assert_eq!(milliseconds(0.5), Some(500));
        assert_eq!(milliseconds(0.25), Some(250));
        assert_eq!(milliseconds(0.0), None);
        assert_eq!(milliseconds(f64::NAN), None);
        assert_eq!(milliseconds(60.0), None);
    }

    #[test]
    fn reads_the_settings_of_the_system() {
        let _ = is_high_contrast();
        assert!(double_click_time() > 0);
    }
}
