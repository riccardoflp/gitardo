//! What a key sends to the shell of a terminal: the bytes a terminal of xterm's kind sends for it, in the modes the
//! program asked for; and the keys that stay the window's.

use model::terminal::Screen;
use ui::{Key, KeyEvent, Modifiers, NamedKey};

use crate::panel::is_panel_key;

/// What a key pressed in a terminal does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyUse {
    /// It sends these bytes to the shell.
    Send(Vec<u8>),
    /// It copies what is selected: `Ctrl+Shift+C`, or `Ctrl+C` while text is selected.
    Copy,
    /// It is the window's: `` Ctrl+` ``, `Ctrl+Tab`, `Ctrl+K`, `Ctrl+Shift+V`, which the window turns into a paste, and
    /// a key alone such as Shift, which sends nothing.
    Window,
    /// It types a character, which comes as text after it.
    Typed,
}

/// What a key does in a terminal showing a screen.
#[must_use]
pub fn key_use(key: KeyEvent, screen: Option<&Screen>) -> KeyUse {
    let modifiers = key.modifiers;
    let control = modifiers.contains(Modifiers::CONTROL);
    let shift = modifiers.contains(Modifiers::SHIFT);
    let alt = modifiers.contains(Modifiers::ALT);
    let is_selected = screen.is_some_and(|screen| screen.selected_text.is_some());
    match key.key {
        Key::Character('c') if control && !alt && (shift || is_selected) => KeyUse::Copy,
        Key::Character('`') if control => KeyUse::Window,
        Key::Character('k' | 'v') if control && !alt => KeyUse::Window,
        _ if is_panel_key(key.key, modifiers) => KeyUse::Window,
        Key::Named(NamedKey::Tab) if control => KeyUse::Window,
        Key::Character(character) if control && !alt => {
            control_byte(character).map_or(KeyUse::Typed, |byte| KeyUse::Send(vec![byte]))
        }
        Key::Character(_) => KeyUse::Typed,
        Key::Named(named) => named_key(
            named,
            modifiers,
            screen.is_some_and(|screen| screen.is_application_cursor),
        ),
    }
}

/// The byte `Ctrl` with a character sends: `Ctrl+C` 3, `Ctrl+[` Escape.
fn control_byte(character: char) -> Option<u8> {
    match character {
        'a'..='z' => u8::try_from(character).ok().map(|byte| byte - b'a' + 1),
        ' ' | '@' | '2' => Some(0),
        '[' | '3' => Some(0x1B),
        '\\' | '4' => Some(0x1C),
        ']' | '5' => Some(0x1D),
        '^' | '6' => Some(0x1E),
        '_' | '-' | '7' => Some(0x1F),
        '8' => Some(0x7F),
        _ => None,
    }
}

/// What a key that does not type sends.
fn named_key(named: NamedKey, modifiers: Modifiers, is_application_cursor: bool) -> KeyUse {
    let modifier = modifier_number(modifiers);
    let bytes: Vec<u8> = match named {
        NamedKey::Enter => b"\r".to_vec(),
        NamedKey::Tab if modifiers.shift() => b"\x1b[Z".to_vec(),
        NamedKey::Tab => b"\t".to_vec(),
        NamedKey::Escape => b"\x1b".to_vec(),
        NamedKey::Backspace if modifiers.contains(Modifiers::CONTROL) => b"\x08".to_vec(),
        NamedKey::Backspace => b"\x7f".to_vec(),
        NamedKey::ArrowUp => cursor_key(b'A', modifier, is_application_cursor),
        NamedKey::ArrowDown => cursor_key(b'B', modifier, is_application_cursor),
        NamedKey::ArrowRight => cursor_key(b'C', modifier, is_application_cursor),
        NamedKey::ArrowLeft => cursor_key(b'D', modifier, is_application_cursor),
        NamedKey::Home => cursor_key(b'H', modifier, is_application_cursor),
        NamedKey::End => cursor_key(b'F', modifier, is_application_cursor),
        NamedKey::Insert => tilde_key(2, modifier),
        NamedKey::Delete => tilde_key(3, modifier),
        NamedKey::PageUp => tilde_key(5, modifier),
        NamedKey::PageDown => tilde_key(6, modifier),
        NamedKey::Function(number) => match function_key(number, modifier) {
            Some(bytes) => bytes,
            None => return KeyUse::Window,
        },
        _ => return KeyUse::Window,
    };
    KeyUse::Send(bytes)
}

/// The number xterm gives the modifiers held with a key, 1 for none: 1 + Shift 1 + Alt 2 + Ctrl 4.
fn modifier_number(modifiers: Modifiers) -> u8 {
    1 + u8::from(modifiers.shift())
        + 2 * u8::from(modifiers.contains(Modifiers::ALT))
        + 4 * u8::from(modifiers.command())
}

/// An arrow, Home or End: `ESC [ A`, `ESC O A` in the mode of the cursor keys, `ESC [ 1 ; 5 A` with Ctrl.
fn cursor_key(letter: u8, modifier: u8, is_application_cursor: bool) -> Vec<u8> {
    if modifier > 1 {
        format!("\x1b[1;{modifier}{}", char::from(letter)).into_bytes()
    } else if is_application_cursor {
        vec![0x1B, b'O', letter]
    } else {
        vec![0x1B, b'[', letter]
    }
}

/// A key sent as a number and a tilde: Delete is `ESC [ 3 ~`, `ESC [ 3 ; 5 ~` with Ctrl.
fn tilde_key(number: u8, modifier: u8) -> Vec<u8> {
    if modifier > 1 {
        format!("\x1b[{number};{modifier}~").into_bytes()
    } else {
        format!("\x1b[{number}~").into_bytes()
    }
}

/// F1 to F12; the others are the window's.
fn function_key(number: u8, modifier: u8) -> Option<Vec<u8>> {
    let letter = match number {
        1 => Some(b'P'),
        2 => Some(b'Q'),
        3 => Some(b'R'),
        4 => Some(b'S'),
        _ => None,
    };
    if let Some(letter) = letter {
        return Some(if modifier > 1 {
            format!("\x1b[1;{modifier}{}", char::from(letter)).into_bytes()
        } else {
            vec![0x1B, b'O', letter]
        });
    }
    let code = match number {
        5 => 15,
        6 => 17,
        7 => 18,
        8 => 19,
        9 => 20,
        10 => 21,
        11 => 23,
        12 => 24,
        _ => return None,
    };
    Some(tilde_key(code, modifier))
}

/// What pasting text sends: the text with its line ends as `Enter` sends them, between the marks of a paste when the
/// program asked for them.
#[must_use]
pub fn paste_bytes(text: &str, screen: Option<&Screen>) -> Vec<u8> {
    let text = text.replace("\r\n", "\r").replace('\n', "\r");
    if screen.is_some_and(|screen| screen.is_bracketed_paste) {
        // The marks themselves are taken out of what is pasted, so that it cannot end the paste early.
        let text = text.replace("\x1b[201~", "");
        format!("\x1b[200~{text}\x1b[201~").into_bytes()
    } else {
        text.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(key: Key, modifiers: Modifiers) -> KeyEvent {
        KeyEvent::press(key, modifiers)
    }

    #[test]
    fn ctrl_c_interrupts_unless_text_is_selected() {
        let ctrl_c = press(Key::Character('c'), Modifiers::CONTROL);
        assert_eq!(key_use(ctrl_c, None), KeyUse::Send(vec![3]));
        let screen = Screen {
            selected_text: Some(String::from("word")),
            ..Screen::default()
        };
        assert_eq!(key_use(ctrl_c, Some(&screen)), KeyUse::Copy);
        let ctrl_shift_c = press(Key::Character('c'), Modifiers::CONTROL | Modifiers::SHIFT);
        assert_eq!(key_use(ctrl_shift_c, None), KeyUse::Copy);
    }

    #[test]
    fn the_keys_of_the_window_stay_the_window_s() {
        for key in [
            press(Key::Character('`'), Modifiers::CONTROL),
            press(Key::Character('`'), Modifiers::CONTROL | Modifiers::SHIFT),
            press(Key::Character('k'), Modifiers::CONTROL),
            press(Key::Named(NamedKey::Tab), Modifiers::CONTROL),
            press(Key::Named(NamedKey::Shift), Modifiers::SHIFT),
        ] {
            assert_eq!(key_use(key, None), KeyUse::Window, "{key:?}");
        }
    }

    #[test]
    fn arrows_follow_the_mode_of_the_cursor_keys() {
        let up = press(Key::Named(NamedKey::ArrowUp), Modifiers::NONE);
        assert_eq!(key_use(up, None), KeyUse::Send(b"\x1b[A".to_vec()));
        let screen = Screen {
            is_application_cursor: true,
            ..Screen::default()
        };
        assert_eq!(key_use(up, Some(&screen)), KeyUse::Send(b"\x1bOA".to_vec()));
        let ctrl_left = press(Key::Named(NamedKey::ArrowLeft), Modifiers::CONTROL);
        assert_eq!(key_use(ctrl_left, None), KeyUse::Send(b"\x1b[1;5D".to_vec()));
    }

    #[test]
    fn enter_tab_and_the_keys_of_editing_send_what_xterm_sends() {
        let send = |named, modifiers| key_use(press(Key::Named(named), modifiers), None);
        assert_eq!(send(NamedKey::Enter, Modifiers::NONE), KeyUse::Send(b"\r".to_vec()));
        assert_eq!(send(NamedKey::Tab, Modifiers::SHIFT), KeyUse::Send(b"\x1b[Z".to_vec()));
        assert_eq!(send(NamedKey::Backspace, Modifiers::NONE), KeyUse::Send(vec![0x7F]));
        assert_eq!(
            send(NamedKey::Delete, Modifiers::NONE),
            KeyUse::Send(b"\x1b[3~".to_vec())
        );
        assert_eq!(
            send(NamedKey::Function(5), Modifiers::NONE),
            KeyUse::Send(b"\x1b[15~".to_vec())
        );
        assert_eq!(
            key_use(press(Key::Character('a'), Modifiers::NONE), None),
            KeyUse::Typed
        );
    }

    #[test]
    fn a_paste_is_marked_when_the_program_asks() {
        assert_eq!(paste_bytes("a\nb", None), b"a\rb".to_vec());
        let screen = Screen {
            is_bracketed_paste: true,
            ..Screen::default()
        };
        assert_eq!(paste_bytes("x", Some(&screen)), b"\x1b[200~x\x1b[201~".to_vec());
    }
}
