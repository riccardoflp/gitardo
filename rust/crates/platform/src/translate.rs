//! The input of `winit` in the words of Gitardo: keys, modifiers, buttons, the wheel, cursors and themes. Each is a
//! pure function, so that it is tested without a window.

use winit::event::{MouseButton, MouseScrollDelta};
use winit::keyboard::{Key as LibraryKey, ModifiersState, NamedKey as LibraryNamedKey};
use winit::window::{CursorIcon, Theme};

use crate::event::Cursor;
use crate::geometry::{Point, Scale};
use crate::input::{Key, Modifiers, NamedKey, PointerButton, WheelDelta};
use crate::system::WheelSteps;
use crate::theme::Appearance;

#[cfg(test)]
mod tests;

/// A key by what it means, from the key the layout gives without modifiers; nothing for keys Gitardo has no name for,
/// such as dead keys and media keys.
pub(crate) fn key(key: &LibraryKey) -> Option<Key> {
    match key {
        LibraryKey::Named(named) => named_key(*named),
        LibraryKey::Character(text) => {
            let mut characters = text.chars();
            let first = characters.next()?;
            let is_one_character = characters.next().is_none();
            is_one_character.then(|| Key::Character(first.to_lowercase().next().unwrap_or(first)))
        }
        LibraryKey::Unidentified(_) | LibraryKey::Dead(_) => None,
    }
}

/// A key that does not type, and the space bar, which `winit` names and Gitardo takes as the space it types.
fn named_key(key: LibraryNamedKey) -> Option<Key> {
    let named = match key {
        LibraryNamedKey::Space => return Some(Key::Character(' ')),
        LibraryNamedKey::Enter => NamedKey::Enter,
        LibraryNamedKey::Tab => NamedKey::Tab,
        LibraryNamedKey::Escape => NamedKey::Escape,
        LibraryNamedKey::Backspace => NamedKey::Backspace,
        LibraryNamedKey::Delete => NamedKey::Delete,
        LibraryNamedKey::Insert => NamedKey::Insert,
        LibraryNamedKey::Home => NamedKey::Home,
        LibraryNamedKey::End => NamedKey::End,
        LibraryNamedKey::PageUp => NamedKey::PageUp,
        LibraryNamedKey::PageDown => NamedKey::PageDown,
        LibraryNamedKey::ArrowLeft => NamedKey::ArrowLeft,
        LibraryNamedKey::ArrowRight => NamedKey::ArrowRight,
        LibraryNamedKey::ArrowUp => NamedKey::ArrowUp,
        LibraryNamedKey::ArrowDown => NamedKey::ArrowDown,
        LibraryNamedKey::Shift => NamedKey::Shift,
        LibraryNamedKey::Control => NamedKey::Control,
        LibraryNamedKey::Alt | LibraryNamedKey::AltGraph => NamedKey::Alt,
        LibraryNamedKey::Super | LibraryNamedKey::Meta => NamedKey::Meta,
        LibraryNamedKey::CapsLock => NamedKey::CapsLock,
        LibraryNamedKey::NumLock => NamedKey::NumLock,
        LibraryNamedKey::ScrollLock => NamedKey::ScrollLock,
        LibraryNamedKey::PrintScreen => NamedKey::PrintScreen,
        LibraryNamedKey::Pause => NamedKey::Pause,
        LibraryNamedKey::ContextMenu => NamedKey::ContextMenu,
        other => NamedKey::Function(function_key(other)?),
    };
    Some(Key::Named(named))
}

/// The number of a function key, F1 to F24.
fn function_key(key: LibraryNamedKey) -> Option<u8> {
    let number = match key {
        LibraryNamedKey::F1 => 1,
        LibraryNamedKey::F2 => 2,
        LibraryNamedKey::F3 => 3,
        LibraryNamedKey::F4 => 4,
        LibraryNamedKey::F5 => 5,
        LibraryNamedKey::F6 => 6,
        LibraryNamedKey::F7 => 7,
        LibraryNamedKey::F8 => 8,
        LibraryNamedKey::F9 => 9,
        LibraryNamedKey::F10 => 10,
        LibraryNamedKey::F11 => 11,
        LibraryNamedKey::F12 => 12,
        LibraryNamedKey::F13 => 13,
        LibraryNamedKey::F14 => 14,
        LibraryNamedKey::F15 => 15,
        LibraryNamedKey::F16 => 16,
        LibraryNamedKey::F17 => 17,
        LibraryNamedKey::F18 => 18,
        LibraryNamedKey::F19 => 19,
        LibraryNamedKey::F20 => 20,
        LibraryNamedKey::F21 => 21,
        LibraryNamedKey::F22 => 22,
        LibraryNamedKey::F23 => 23,
        LibraryNamedKey::F24 => 24,
        // The hundreds of other keys `winit` names (media, browser, television keys…) are no keys of Gitardo.
        _ => return None,
    };
    Some(number)
}

/// The modifier a key is, if it is one.
pub(crate) fn modifier_of(key: Key) -> Modifiers {
    match key {
        Key::Named(NamedKey::Shift) => Modifiers::SHIFT,
        Key::Named(NamedKey::Control) => Modifiers::CONTROL,
        Key::Named(NamedKey::Alt) => Modifiers::ALT,
        Key::Named(NamedKey::Meta) => Modifiers::META,
        Key::Named(_) | Key::Character(_) => Modifiers::NONE,
    }
}

/// The text a key typed, without the control characters that Enter, Tab, Backspace, Escape and Ctrl with a letter
/// give: those are keys, not text. Nothing when no text is left.
pub(crate) fn typed_text(text: &str) -> Option<String> {
    let typed: String = text.chars().filter(|character| !character.is_control()).collect();
    (!typed.is_empty()).then_some(typed)
}

/// The modifier keys held.
pub(crate) fn modifiers(state: ModifiersState) -> Modifiers {
    let pairs = [
        (state.shift_key(), Modifiers::SHIFT),
        (state.control_key(), Modifiers::CONTROL),
        (state.alt_key(), Modifiers::ALT),
        (state.super_key(), Modifiers::META),
    ];
    pairs
        .into_iter()
        .filter(|(is_held, _)| *is_held)
        .fold(Modifiers::NONE, |held, (_, modifier)| held | modifier)
}

/// A button of the pointer; nothing for the buttons beyond the five Gitardo knows.
pub(crate) fn button(button: MouseButton) -> Option<PointerButton> {
    match button {
        MouseButton::Left => Some(PointerButton::Primary),
        MouseButton::Right => Some(PointerButton::Secondary),
        MouseButton::Middle => Some(PointerButton::Middle),
        MouseButton::Back => Some(PointerButton::Back),
        MouseButton::Forward => Some(PointerButton::Forward),
        MouseButton::Other(_) => None,
    }
}

/// How far to scroll, the other way round from `winit`: it says where the content moves, Gitardo where the view
/// goes. A notch of the wheel is the lines and characters the system's settings say; pixels are logical.
pub(crate) fn wheel_delta(delta: MouseScrollDelta, steps: WheelSteps, scale: Scale) -> WheelDelta {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => WheelDelta::Lines {
            x: -x * steps.across,
            y: -y * steps.down,
        },
        MouseScrollDelta::PixelDelta(position) => {
            let logical = logical_point(position.x, position.y, scale);
            WheelDelta::Pixels {
                x: -logical.x,
                y: -logical.y,
            }
        }
    }
}

/// A point of the system in physical pixels, in logical pixels.
#[allow(
    clippy::cast_possible_truncation,
    reason = "positions on a screen are far inside the range of f32, and a cast of a float saturates"
)]
pub(crate) fn logical_point(x: f64, y: f64, scale: Scale) -> Point {
    let factor = f64::from(scale.factor());
    Point {
        x: (x / factor) as f32,
        y: (y / factor) as f32,
    }
}

/// The shape `winit` shows for a cursor of Gitardo.
pub(crate) fn cursor_icon(cursor: Cursor) -> CursorIcon {
    match cursor {
        Cursor::Arrow => CursorIcon::Default,
        Cursor::Text => CursorIcon::Text,
        Cursor::Hand => CursorIcon::Pointer,
        Cursor::ResizeEastWest => CursorIcon::EwResize,
        Cursor::ResizeNorthSouth => CursorIcon::NsResize,
        Cursor::Grab => CursorIcon::Grab,
        Cursor::NotAllowed => CursorIcon::NotAllowed,
    }
}

/// Dark or light, from the theme of `winit`.
pub(crate) fn appearance(theme: Theme) -> Appearance {
    match theme {
        Theme::Light => Appearance::Light,
        Theme::Dark => Appearance::Dark,
    }
}

/// The theme of `winit` for dark or light.
pub(crate) fn library_theme(appearance: Appearance) -> Theme {
    match appearance {
        Appearance::Light => Theme::Light,
        Appearance::Dark => Theme::Dark,
    }
}
