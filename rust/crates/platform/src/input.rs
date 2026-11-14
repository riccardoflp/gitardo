//! What a window receives from the person using it, and from the system about the window itself.

use std::ops::{BitOr, BitOrAssign};

use crate::accessibility::AccessibilityRequest;
use crate::geometry::{PhysicalSize, Point, Scale};

/// Something that happened to a window: input from the person, or a change the system made.
#[derive(Clone, Debug, PartialEq)]
pub enum InputEvent {
    /// The pointer moved over the window, or anywhere while a button is held after a press in it.
    PointerMoved(PointerMove),
    /// A button of the pointer was pressed in the window.
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
    /// The input method is composing this text, not typed yet: it is shown at the caret until it is typed, or
    /// empty once the composition ends.
    TextComposing(String),
    /// The window got the keyboard.
    FocusGained,
    /// The window lost the keyboard.
    FocusLost,
    /// The inside of the window has a new size, in physical pixels.
    Resized(PhysicalSize),
    /// The window moved to a screen of another scale, or the scale of its screen changed.
    ScaleChanged(Scale),
    /// The person asked to close the window. It stays open until the program closes it.
    CloseRequested,
    /// The window needs a new frame: it was resized, uncovered with nothing to show, or a redraw was asked for.
    RedrawRequested,
    /// A screen reader asks for something: the tree of what is on screen, or an action on a part of it.
    Accessibility(AccessibilityRequest),
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
    /// 1 for a click, 2 for a double click, 3 for a triple click, and so on; a release has the count of its press.
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
    /// The modifier keys held, the key itself included when it is one.
    pub modifiers: Modifiers,
    /// Whether this press repeats because the key is held; always false for a release.
    pub is_repeat: bool,
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

#[cfg(test)]
mod tests {
    use super::Modifiers;

    #[test]
    fn combines_modifiers() {
        let mut held = Modifiers::SHIFT | Modifiers::CONTROL;
        assert!(held.contains(Modifiers::SHIFT));
        assert!(held.contains(Modifiers::SHIFT | Modifiers::CONTROL));
        assert!(!held.contains(Modifiers::ALT));
        assert!(held.contains(Modifiers::NONE));
        held |= Modifiers::META;
        assert!(held.contains(Modifiers::META));
        assert!(Modifiers::NONE.is_empty());
        assert!(!held.is_empty());
    }
}
