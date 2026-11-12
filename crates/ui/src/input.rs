//! What the toolkit is told happened since the last frame: the pointer, its buttons and wheel, the keys, the text
//! typed or pasted, and whether the window has the keyboard.
//!
//! These types follow the input events of the platform one for one, so that the application translates them with a
//! `match` and nothing more. The toolkit does not depend on the platform: it can be driven by tests without a window.

use std::ops::{BitOr, BitOrAssign};

use render::Point;

/// Something that happened in the window since the last frame, in the order it happened.
#[derive(Clone, Debug, PartialEq)]
pub enum InputEvent {
    /// The pointer moved, to a place in logical pixels inside the window.
    PointerMoved(PointerMove),
    /// A button of the pointer was pressed.
    PointerPressed(PointerButtonEvent),
    /// A button of the pointer was released.
    PointerReleased(PointerButtonEvent),
    /// The wheel turned, or a touchpad scrolled.
    Wheel(WheelEvent),
    /// The pointer left the window.
    PointerLeft,
    /// A key went down, or repeats while held.
    KeyPressed(KeyEvent),
    /// A key went up.
    KeyReleased(KeyEvent),
    /// Text was typed, after the keyboard layout and the input method made it.
    TextTyped(String),
    /// Text to paste where the keyboard is: the application reads the clipboard when Ctrl+V is pressed and hands its
    /// text in, since the toolkit never asks the system.
    Paste(String),
    /// The window got the keyboard.
    FocusGained,
    /// The window lost the keyboard.
    FocusLost,
}

/// Where the pointer moved to.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PointerMove {
    /// Where the pointer is, in logical pixels inside the window.
    pub position: Point,
    /// The modifier keys held.
    pub modifiers: Modifiers,
}

/// A button of the pointer pressed or released.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerButtonEvent {
    /// The button.
    pub button: PointerButton,
    /// Where the pointer is, in logical pixels inside the window.
    pub position: Point,
    /// The modifier keys held.
    pub modifiers: Modifiers,
    /// 1 for a click, 2 for a double click, 3 for a triple click; a release has the count of its press. The system
    /// counts them, with its own setting for the time between clicks.
    pub click_count: u32,
}

/// A button of a mouse or a touchpad.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PointerButton {
    /// The main button, usually the left one.
    Primary,
    /// The button that opens context menus, usually the right one.
    Secondary,
    /// The wheel pressed, or the middle button.
    Middle,
    /// The side button that goes back.
    Back,
    /// The side button that goes forward.
    Forward,
}

/// A turn of the wheel, or a scroll on a touchpad.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelEvent {
    /// How far to scroll.
    pub delta: WheelDelta,
    /// Where the pointer is, in logical pixels inside the window.
    pub position: Point,
    /// The modifier keys held.
    pub modifiers: Modifiers,
}

/// How far to scroll. Positive `x` scrolls toward the right end of the content, positive `y` toward its bottom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WheelDelta {
    /// In lines of text, as the system's setting for the wheel says: what a mouse wheel gives.
    Lines {
        /// Across.
        x: f32,
        /// Down.
        y: f32,
    },
    /// In logical pixels: what a touchpad gives on the systems that say so.
    Pixels {
        /// Across.
        x: f32,
        /// Down.
        y: f32,
    },
}

/// A key pressed or released.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    /// The key.
    pub key: Key,
    /// The modifier keys held.
    pub modifiers: Modifiers,
    /// Whether this press repeats because the key is held; always false for a release.
    pub is_repeat: bool,
}

impl KeyEvent {
    /// A press of a key with modifiers, not a repeat.
    #[must_use]
    pub fn press(key: Key, modifiers: Modifiers) -> KeyEvent {
        KeyEvent {
            key,
            modifiers,
            is_repeat: false,
        }
    }

    /// Whether this is a key with exactly these modifiers held.
    #[must_use]
    pub fn is(self, key: Key, modifiers: Modifiers) -> bool {
        self.key == key && self.modifiers == modifiers
    }
}

/// A key of the keyboard, by what it means rather than where it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    /// A key that does not type, by its name.
    Named(NamedKey),
    /// A key that types, by the character it types without Shift in the current layout, in lowercase: `a`, `1`,
    /// `,`, a space.
    Character(char),
}

/// The keys that do not type a character.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NamedKey {
    /// Enter, or Return.
    Enter,
    /// Tab.
    Tab,
    /// Escape.
    Escape,
    /// Backspace.
    Backspace,
    /// Delete.
    Delete,
    /// Insert.
    Insert,
    /// Home.
    Home,
    /// End.
    End,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// The left arrow.
    ArrowLeft,
    /// The right arrow.
    ArrowRight,
    /// The up arrow.
    ArrowUp,
    /// The down arrow.
    ArrowDown,
    /// A function key, F1 to F24, by its number.
    Function(u8),
    /// Shift, left or right.
    Shift,
    /// Control, left or right.
    Control,
    /// Alt, or Option.
    Alt,
    /// The Windows key, or Command.
    Meta,
    /// Caps lock.
    CapsLock,
    /// Num lock.
    NumLock,
    /// Scroll lock.
    ScrollLock,
    /// Print screen.
    PrintScreen,
    /// Pause, or Break.
    Pause,
    /// The key that opens the context menu.
    ContextMenu,
}

/// The modifier keys held, as a set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers {
    /// One bit for each key.
    keys: u8,
}

impl Modifiers {
    /// No modifier key.
    pub const NONE: Modifiers = Modifiers { keys: 0 };
    /// Shift.
    pub const SHIFT: Modifiers = Modifiers { keys: 1 };
    /// Control.
    pub const CONTROL: Modifiers = Modifiers { keys: 2 };
    /// Alt, or Option.
    pub const ALT: Modifiers = Modifiers { keys: 4 };
    /// The Windows key, or Command.
    pub const META: Modifiers = Modifiers { keys: 8 };

    /// Whether every key of `other` is held.
    #[must_use]
    pub fn contains(self, other: Modifiers) -> bool {
        self.keys & other.keys == other.keys
    }

    /// Whether no modifier key is held.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.keys == 0
    }

    /// Whether Shift is held.
    #[must_use]
    pub fn shift(self) -> bool {
        self.contains(Modifiers::SHIFT)
    }

    /// Whether the key of commands is held: Control, or Command on macOS, which the application maps to Control.
    #[must_use]
    pub fn command(self) -> bool {
        self.contains(Modifiers::CONTROL)
    }
}

impl BitOr for Modifiers {
    type Output = Modifiers;

    fn bitor(self, other: Modifiers) -> Modifiers {
        Modifiers {
            keys: self.keys | other.keys,
        }
    }
}

impl BitOrAssign for Modifiers {
    fn bitor_assign(&mut self, other: Modifiers) {
        self.keys |= other.keys;
    }
}

/// The shape of the pointer the toolkit asks for, as the platform names them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Cursor {
    /// The arrow.
    #[default]
    Arrow,
    /// The text beam, over text that can be selected or edited.
    Text,
    /// The hand, over a link.
    Hand,
    /// The arrows left and right, over something that resizes across.
    ResizeEastWest,
    /// The arrows up and down, over something that resizes down.
    ResizeNorthSouth,
    /// The shape for dragging something around.
    Grab,
    /// The sign that what the pointer is over cannot be used now.
    NotAllowed,
}
