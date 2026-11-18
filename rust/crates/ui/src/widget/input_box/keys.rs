//! What the keys do in an input box: the arrows, Home and End move the caret, with Shift to select and Ctrl to go by
//! words; Backspace and Delete remove, by words with Ctrl; Ctrl+A selects all; Ctrl+C and Ctrl+X copy and cut;
//! Ctrl+Z undoes, Ctrl+Y and Ctrl+Shift+Z redo; Enter submits. Ctrl+V is the application's: it reads the clipboard
//! and hands its text in as a paste.

use super::line::{EditableLine, Extent, Movement, Selecting};
use crate::input::{Key, KeyEvent, Modifiers, NamedKey};

/// What a key did that the box tells others.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct KeyOutcome {
    /// Whether Enter was pressed.
    pub(crate) is_submitted: bool,
    /// Text copied or cut, for the clipboard.
    pub(crate) copied: Option<String>,
}

/// Whether an input box uses a key, so that it is not a shortcut of the view: the keys that move, select and delete,
/// those of the clipboard and of undo, and every key that types. Enter, Escape, Tab, the arrows up and down and the
/// function keys are left to the view and to what holds the box, such as a dialog or a picker.
pub(crate) fn is_used(key: KeyEvent) -> bool {
    let modifiers = key.modifiers;
    match key.key {
        Key::Named(named) => match named {
            NamedKey::ArrowLeft
            | NamedKey::ArrowRight
            | NamedKey::Home
            | NamedKey::End
            | NamedKey::Backspace
            | NamedKey::Delete => true,
            NamedKey::Insert => modifiers == Modifiers::SHIFT || modifiers == Modifiers::CONTROL,
            _ => false,
        },
        Key::Character(character) if modifiers.command() && !modifiers.contains(Modifiers::ALT) => {
            matches!(
                (character, modifiers.shift()),
                ('a' | 'c' | 'x' | 'v' | 'z' | 'y', false) | ('z', true)
            )
        }
        Key::Character(_) => types(modifiers),
    }
}

/// Whether a key of a character types with these modifier keys held. Not with the Windows or Command key; and not
/// with `Alt` alone, which is a shortcut of the view, while with `Ctrl` too it types, as `AltGr` does. On macOS
/// `Option` with a key types a character, and the shortcut of the view is `Ctrl` and `Option` with the key.
fn types(modifiers: Modifiers) -> bool {
    let (is_alt, is_control) = (
        modifiers.contains(Modifiers::ALT),
        modifiers.contains(Modifiers::CONTROL),
    );
    let is_shortcut = if cfg!(target_os = "macos") {
        is_alt && is_control
    } else {
        is_alt && !is_control
    };
    !modifiers.contains(Modifiers::META) && !is_shortcut
}

/// Applies a key to the line of an input box.
pub(crate) fn apply_key(line: &mut EditableLine, key: KeyEvent) -> KeyOutcome {
    let mut outcome = KeyOutcome::default();
    let modifiers = key.modifiers;
    let selecting = if modifiers.shift() {
        Selecting::Extend
    } else {
        Selecting::Move
    };
    let by_word = modifiers.command();
    let extent = if by_word { Extent::Word } else { Extent::Character };
    match key.key {
        Key::Named(named) => match named {
            NamedKey::ArrowLeft => {
                line.move_caret(if by_word { Movement::WordLeft } else { Movement::Left }, selecting);
            }
            NamedKey::ArrowRight => {
                line.move_caret(if by_word { Movement::WordRight } else { Movement::Right }, selecting);
            }
            NamedKey::Home => line.move_caret(Movement::Home, selecting),
            NamedKey::End => line.move_caret(Movement::End, selecting),
            NamedKey::Backspace => {
                let _ = line.delete_backward(extent);
            }
            NamedKey::Delete => {
                let _ = line.delete_forward(extent);
            }
            NamedKey::Enter => outcome.is_submitted = modifiers.is_empty(),
            _ => {}
        },
        Key::Character(character) if modifiers.command() && !modifiers.contains(Modifiers::ALT) => {
            match (character, modifiers.shift()) {
                ('a', false) => line.select_all(),
                ('c', false) => outcome.copied = Some(line.selected_text().to_owned()).filter(|text| !text.is_empty()),
                ('x', false) => outcome.copied = line.cut(),
                ('z', false) => {
                    let _ = line.undo();
                }
                ('y', false) | ('z', true) => {
                    let _ = line.redo();
                }
                _ => {}
            }
        }
        Key::Character(_) => {}
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(line: &mut EditableLine, key: Key, modifiers: Modifiers) -> KeyOutcome {
        apply_key(line, KeyEvent::press(key, modifiers))
    }

    #[test]
    fn the_keys_of_editing_move_select_copy_cut_and_undo() {
        let mut line = EditableLine::new("git fetch --all");
        let _ = press(
            &mut line,
            Key::Named(NamedKey::ArrowLeft),
            Modifiers::CONTROL | Modifiers::SHIFT,
        );
        assert_eq!(line.selected_text(), "all");
        let outcome = press(&mut line, Key::Character('c'), Modifiers::CONTROL);
        assert_eq!(outcome.copied.as_deref(), Some("all"));
        let outcome = press(&mut line, Key::Character('x'), Modifiers::CONTROL);
        assert_eq!((outcome.copied.as_deref(), line.text()), (Some("all"), "git fetch --"));
        let _ = press(&mut line, Key::Named(NamedKey::Backspace), Modifiers::CONTROL);
        assert_eq!(line.text(), "git fetch ");
        let _ = press(&mut line, Key::Character('z'), Modifiers::CONTROL);
        let _ = press(&mut line, Key::Character('z'), Modifiers::CONTROL);
        assert_eq!(line.text(), "git fetch --all");
        let _ = press(&mut line, Key::Character('z'), Modifiers::CONTROL | Modifiers::SHIFT);
        assert_eq!(line.text(), "git fetch --");
        let _ = press(&mut line, Key::Named(NamedKey::Home), Modifiers::SHIFT);
        assert_eq!(line.selected_text(), "git fetch --");
        let _ = press(&mut line, Key::Character('a'), Modifiers::CONTROL);
        let outcome = press(&mut line, Key::Named(NamedKey::Enter), Modifiers::NONE);
        assert!(outcome.is_submitted);
        assert_eq!(
            press(&mut line, Key::Character('c'), Modifiers::NONE),
            KeyOutcome::default()
        );
    }

    #[test]
    fn a_key_with_alt_alone_is_left_to_the_view_and_on_macos_one_with_ctrl_and_option() {
        let on_macos = cfg!(target_os = "macos");
        let used = |modifiers: Modifiers| is_used(KeyEvent::press(Key::Character('c'), modifiers));
        assert!(used(Modifiers::NONE));
        assert!(used(Modifiers::SHIFT));
        assert_eq!(used(Modifiers::ALT), on_macos);
        assert_eq!(used(Modifiers::ALT | Modifiers::CONTROL), !on_macos);
        assert!(!used(Modifiers::META));
    }
}
