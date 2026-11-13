//! What the keys do in an editor of code: the arrows, Home, End, Page Up and Page Down move the caret, with Shift to
//! select and Ctrl to go by words or to the ends of the text; Backspace and Delete remove, by words with Ctrl; Enter
//! and Tab type; Ctrl+A selects all; Ctrl+C and Ctrl+X copy and cut; Ctrl+Z undoes, Ctrl+Y and Ctrl+Shift+Z redo. In
//! an editor read only, the keys that change the text are left to the view.

use editor::{Command, Extent, Movement};

use crate::input::{Key, KeyEvent, Modifiers, NamedKey};

/// What a key asks of an editor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct KeyCommands {
    /// Whether the editor uses the key, so that it is not a shortcut of the view.
    pub(crate) is_used: bool,
    /// Whether the selection is copied to the clipboard first.
    pub(crate) copies: bool,
    /// What it asks of the document.
    pub(crate) commands: Vec<Command>,
}

/// What a key asks of an editor, which goes a page of so many lines with Page Up and Page Down.
pub(crate) fn key_commands(key: KeyEvent, page: usize, is_read_only: bool) -> KeyCommands {
    let asked = match key.key {
        Key::Named(named) => named_key(named, key.modifiers, page),
        Key::Character(character) => character_key(character, key.modifiers),
    };
    if is_read_only && asked.commands.iter().any(Command::is_edit) {
        return KeyCommands {
            is_used: asked.copies,
            copies: asked.copies,
            commands: Vec::new(),
        };
    }
    asked
}

/// A key that asks one command.
fn one(command: Command) -> KeyCommands {
    KeyCommands {
        is_used: true,
        copies: false,
        commands: vec![command],
    }
}

/// What a named key asks: the keys that move and select, delete, and type a line break or a tab.
fn named_key(named: NamedKey, modifiers: Modifiers, page: usize) -> KeyCommands {
    let is_extending = modifiers.shift();
    let by_word = modifiers.command();
    let movement = match named {
        NamedKey::ArrowLeft if by_word => Some(Movement::WordLeft),
        NamedKey::ArrowLeft => Some(Movement::Left),
        NamedKey::ArrowRight if by_word => Some(Movement::WordRight),
        NamedKey::ArrowRight => Some(Movement::Right),
        NamedKey::ArrowUp if !by_word => Some(Movement::Up(1)),
        NamedKey::ArrowDown if !by_word => Some(Movement::Down(1)),
        NamedKey::PageUp => Some(Movement::Up(page)),
        NamedKey::PageDown => Some(Movement::Down(page)),
        NamedKey::Home if by_word => Some(Movement::TextStart),
        NamedKey::Home => Some(Movement::LineStart),
        NamedKey::End if by_word => Some(Movement::TextEnd),
        NamedKey::End => Some(Movement::LineEnd),
        _ => None,
    };
    if let Some(movement) = movement {
        return one(Command::Move(movement, is_extending));
    }
    let extent = if by_word { Extent::Word } else { Extent::Character };
    match named {
        NamedKey::Backspace => one(Command::DeleteBackward(extent)),
        NamedKey::Delete => one(Command::DeleteForward(extent)),
        NamedKey::Enter if modifiers.is_empty() => one(Command::NewLine),
        NamedKey::Tab if modifiers.is_empty() => one(Command::Type("\t".to_owned())),
        _ => KeyCommands::default(),
    }
}

/// What a key that types a character asks with Ctrl: select all, copy, cut, undo and redo.
fn character_key(character: char, modifiers: Modifiers) -> KeyCommands {
    if !modifiers.command() || modifiers.contains(Modifiers::ALT) {
        return KeyCommands::default();
    }
    match (character, modifiers.shift()) {
        ('a', false) => one(Command::SelectAll),
        ('c', false) => KeyCommands {
            is_used: true,
            copies: true,
            commands: Vec::new(),
        },
        ('x', false) => KeyCommands {
            is_used: true,
            copies: true,
            commands: vec![Command::DeleteSelection],
        },
        ('z', false) => one(Command::Undo),
        ('y', false) | ('z', true) => one(Command::Redo),
        _ => KeyCommands::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(key: Key, modifiers: Modifiers, is_read_only: bool) -> KeyCommands {
        key_commands(KeyEvent::press(key, modifiers), 20, is_read_only)
    }

    #[test]
    fn the_keys_of_editing_move_select_delete_and_undo() {
        let cases = [
            (
                Key::Named(NamedKey::ArrowLeft),
                Modifiers::CONTROL | Modifiers::SHIFT,
                Command::Move(Movement::WordLeft, true),
            ),
            (
                Key::Named(NamedKey::PageDown),
                Modifiers::NONE,
                Command::Move(Movement::Down(20), false),
            ),
            (
                Key::Named(NamedKey::Home),
                Modifiers::CONTROL,
                Command::Move(Movement::TextStart, false),
            ),
            (
                Key::Named(NamedKey::Backspace),
                Modifiers::CONTROL,
                Command::DeleteBackward(Extent::Word),
            ),
            (Key::Named(NamedKey::Enter), Modifiers::NONE, Command::NewLine),
            (
                Key::Character('z'),
                Modifiers::CONTROL | Modifiers::SHIFT,
                Command::Redo,
            ),
        ];
        for (key, modifiers, command) in cases {
            assert_eq!(press(key, modifiers, false).commands, [command], "{key:?}");
        }
        let cut = press(Key::Character('x'), Modifiers::CONTROL, false);
        assert!(cut.copies);
        assert_eq!(cut.commands, [Command::DeleteSelection]);
    }

    #[test]
    fn a_read_only_editor_moves_and_copies_but_does_not_edit() {
        assert_eq!(
            press(Key::Named(NamedKey::Enter), Modifiers::NONE, true),
            KeyCommands::default()
        );
        let copy = press(Key::Character('x'), Modifiers::CONTROL, true);
        assert!(copy.copies && copy.commands.is_empty());
        assert_eq!(
            press(Key::Named(NamedKey::ArrowDown), Modifiers::SHIFT, true).commands,
            [Command::Move(Movement::Down(1), true)]
        );
        assert!(!press(Key::Named(NamedKey::Escape), Modifiers::NONE, false).is_used);
        assert!(!press(Key::Character('f'), Modifiers::CONTROL, false).is_used);
    }
}
