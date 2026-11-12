//! What the keys and the characters of Windows mean: virtual keys as `Key`s, and UTF-16 units joined into characters.

use super::user32::{
    VK_ADD, VK_APPS, VK_BACK, VK_CAPITAL, VK_CONTROL, VK_DECIMAL, VK_DELETE, VK_DIVIDE, VK_DOWN, VK_END, VK_ESCAPE,
    VK_F1, VK_F24, VK_HOME, VK_INSERT, VK_LEFT, VK_LWIN, VK_MENU, VK_MULTIPLY, VK_NEXT, VK_NUMLOCK, VK_NUMPAD0,
    VK_NUMPAD9, VK_PAUSE, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_RWIN, VK_SCROLL, VK_SHIFT, VK_SNAPSHOT, VK_SPACE,
    VK_SUBTRACT, VK_TAB, VK_UP,
};
use crate::input::{Key, NamedKey};

/// The bit of the result of `MapVirtualKeyW` that marks a dead key, one that changes the next character.
const DEAD_KEY: u32 = 0x8000_0000;

/// The key a virtual key is. `mapped` is what `MapVirtualKeyW` gives for it with `MAPVK_VK_TO_CHAR`: the character
/// the key types without Shift in the current layout, or 0 for none. `None` for keys Gitardo does not use.
pub(crate) fn key_from_virtual_key(virtual_key: u16, mapped: u32) -> Option<Key> {
    if let Some(named) = named_key(virtual_key) {
        return Some(Key::Named(named));
    }
    if let Some(character) = unmapped_character(virtual_key) {
        return Some(Key::Character(character));
    }
    let character =
        char::from_u32(mapped & !DEAD_KEY).filter(|character| *character != '\0' && !character.is_control())?;
    Some(Key::Character(character.to_lowercase().next().unwrap_or(character)))
}

/// The key that does not type, for a virtual key that is one.
fn named_key(virtual_key: u16) -> Option<NamedKey> {
    let named = match virtual_key {
        VK_RETURN => NamedKey::Enter,
        VK_TAB => NamedKey::Tab,
        VK_ESCAPE => NamedKey::Escape,
        VK_BACK => NamedKey::Backspace,
        VK_DELETE => NamedKey::Delete,
        VK_INSERT => NamedKey::Insert,
        VK_HOME => NamedKey::Home,
        VK_END => NamedKey::End,
        VK_PRIOR => NamedKey::PageUp,
        VK_NEXT => NamedKey::PageDown,
        VK_LEFT => NamedKey::ArrowLeft,
        VK_RIGHT => NamedKey::ArrowRight,
        VK_UP => NamedKey::ArrowUp,
        VK_DOWN => NamedKey::ArrowDown,
        VK_SHIFT => NamedKey::Shift,
        VK_CONTROL => NamedKey::Control,
        VK_MENU => NamedKey::Alt,
        VK_LWIN | VK_RWIN => NamedKey::Meta,
        VK_CAPITAL => NamedKey::CapsLock,
        VK_NUMLOCK => NamedKey::NumLock,
        VK_SCROLL => NamedKey::ScrollLock,
        VK_SNAPSHOT => NamedKey::PrintScreen,
        VK_PAUSE => NamedKey::Pause,
        VK_APPS => NamedKey::ContextMenu,
        VK_F1..=VK_F24 => NamedKey::Function(u8::try_from(virtual_key - VK_F1 + 1).ok()?),
        // Every other virtual key types a character, or is one Gitardo does not use.
        _ => return None,
    };
    Some(named)
}

/// The character of the space bar and of the keys of the numeric pad, which the layout does not map: the same as the
/// other keys that type it, so that a shortcut works with either.
fn unmapped_character(virtual_key: u16) -> Option<char> {
    match virtual_key {
        VK_NUMPAD0..=VK_NUMPAD9 => char::from_digit(u32::from(virtual_key - VK_NUMPAD0), 10),
        VK_MULTIPLY => Some('*'),
        VK_ADD => Some('+'),
        VK_SUBTRACT => Some('-'),
        VK_DECIMAL => Some('.'),
        VK_DIVIDE => Some('/'),
        VK_SPACE => Some(' '),
        _ => None,
    }
}

/// Whether a character typed is text, rather than a control character a key also makes, such as Backspace or
/// Control and a letter.
pub(crate) fn is_text(character: char) -> bool {
    !character.is_control()
}

/// Joins the two halves of a character outside the Basic Multilingual Plane, which `WM_CHAR` brings one at a time.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Utf16Joiner {
    high: Option<u16>,
}

impl Utf16Joiner {
    /// The character a unit completes, if any. A half without its other half is dropped.
    pub(crate) fn push(&mut self, unit: u16) -> Option<char> {
        match unit {
            0xD800..=0xDBFF => {
                self.high = Some(unit);
                None
            }
            0xDC00..=0xDFFF => {
                let high = self.high.take()?;
                let value = 0x1_0000 + ((u32::from(high) - 0xD800) << 10) + (u32::from(unit) - 0xDC00);
                char::from_u32(value)
            }
            _ => {
                self.high = None;
                char::from_u32(u32::from(unit))
            }
        }
    }

    /// Forgets a half waiting for its other half, when the window loses the keyboard.
    pub(crate) fn clear(&mut self) {
        self.high = None;
    }
}

#[cfg(test)]
mod tests {
    use super::{Utf16Joiner, is_text, key_from_virtual_key};
    use crate::input::{Key, NamedKey};

    #[test]
    fn maps_virtual_keys() {
        let cases = [
            (0x0D, 0x0D, Some(Key::Named(NamedKey::Enter))),
            (0x08, 0x08, Some(Key::Named(NamedKey::Backspace))),
            (0x25, 0, Some(Key::Named(NamedKey::ArrowLeft))),
            (0x70, 0, Some(Key::Named(NamedKey::Function(1)))),
            (0x87, 0, Some(Key::Named(NamedKey::Function(24)))),
            (0x5C, 0, Some(Key::Named(NamedKey::Meta))),
            (0x41, u32::from('A'), Some(Key::Character('a'))),
            (0x31, u32::from('1'), Some(Key::Character('1'))),
            (0xBC, u32::from(','), Some(Key::Character(','))),
            (0x20, u32::from(' '), Some(Key::Character(' '))),
            (0x63, 0, Some(Key::Character('3'))),
            (0x6B, 0, Some(Key::Character('+'))),
            (0xBA, 0x8000_0000 | u32::from('^'), Some(Key::Character('^'))),
            (0xBA, u32::from('È'), Some(Key::Character('è'))),
            (0xFF, 0, None),
        ];
        for (virtual_key, mapped, expected) in cases {
            assert_eq!(key_from_virtual_key(virtual_key, mapped), expected, "{virtual_key:#x}");
        }
    }

    #[test]
    fn joins_surrogate_pairs() {
        let mut joiner = Utf16Joiner::default();
        assert_eq!(joiner.push(u16::from(b'a')), Some('a'));
        assert_eq!(joiner.push(0xD83D), None);
        assert_eq!(joiner.push(0xDE00), Some('😀'));
        assert_eq!(joiner.push(0xDE00), None);
        assert_eq!(joiner.push(0xD83D), None);
        assert_eq!(joiner.push(u16::from(b'b')), Some('b'));
        assert_eq!(joiner.push(0xDE00), None);
        assert_eq!(joiner.push(0xD83D), None);
        joiner.clear();
        assert_eq!(joiner.push(0xDE00), None);
    }

    #[test]
    fn tells_text_from_control_characters() {
        assert!(is_text('a'));
        assert!(is_text('😀'));
        assert!(!is_text('\u{8}'));
        assert!(!is_text('\u{1}'));
        assert!(!is_text('\u{7f}'));
    }
}
