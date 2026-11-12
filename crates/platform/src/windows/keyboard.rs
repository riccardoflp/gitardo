//! The keyboard messages of a window: keys pressed and released, characters typed, and the text an input method
//! commits.

use super::handle::WindowHandle;
use super::keys::{is_text, key_from_virtual_key};
use super::procedure::{Message, WindowState};
use super::types::LRESULT;
use super::{ime, imm32, low_word, user32};
use crate::input::{InputEvent, KeyEvent, Modifiers};

/// The bit of the `LPARAM` of a key message that says the key was already down: the press repeats.
const PREVIOUS_KEY_STATE: isize = 1 << 30;

/// What a keyboard message does; `None` for the messages the system handles alone.
pub(crate) fn keyboard_message(state: &WindowState, window: WindowHandle, message: Message) -> Option<LRESULT> {
    match message.id {
        user32::WM_KEYDOWN => {
            pressed(state, message);
            Some(0)
        }
        user32::WM_SYSKEYDOWN => {
            pressed(state, message);
            // Alt and F4 is left to the system, which turns it into WM_CLOSE. Other keys with Alt are not, so that
            // F10 or Alt alone does not move the keyboard to a menu Gitardo does not have.
            (low_word(message.wparam) != user32::VK_F4).then_some(0)
        }
        user32::WM_KEYUP | user32::WM_SYSKEYUP => {
            released(state, message);
            Some(0)
        }
        user32::WM_CHAR => {
            typed(state, message);
            Some(0)
        }
        // Alt and Space opens the window's menu of the system; other characters with Alt are shortcuts, not text.
        user32::WM_SYSCHAR => (low_word(message.wparam) != u16::from(b' ')).then_some(0),
        user32::WM_IME_COMPOSITION => committed(state, window, message),
        _ => None,
    }
}

/// `WM_KEYDOWN`: a key went down, or repeats.
fn pressed(state: &WindowState, message: Message) {
    if let Some(event) = key_event(message) {
        let is_repeat = message.lparam & PREVIOUS_KEY_STATE != 0;
        state.push(InputEvent::KeyPressed(KeyEvent { is_repeat, ..event }));
    }
}

/// `WM_KEYUP`: a key went up.
fn released(state: &WindowState, message: Message) {
    if let Some(event) = key_event(message) {
        state.push(InputEvent::KeyReleased(event));
    }
}

/// The key of a key message, with the modifiers held.
fn key_event(message: Message) -> Option<KeyEvent> {
    let virtual_key = low_word(message.wparam);
    // SAFETY: only integers are passed.
    let mapped = unsafe { user32::MapVirtualKeyW(u32::from(virtual_key), user32::MAPVK_VK_TO_CHAR) };
    let key = key_from_virtual_key(virtual_key, mapped)?;
    Some(KeyEvent {
        key,
        modifiers: modifiers_held(),
        is_repeat: false,
    })
}

/// `WM_CHAR`: a character typed, or half of one.
fn typed(state: &WindowState, message: Message) {
    let character = state.keyboard.borrow_mut().push(low_word(message.wparam));
    if let Some(character) = character.filter(|character| is_text(*character)) {
        state.push(InputEvent::TextTyped(character.to_string()));
    }
}

/// `WM_IME_COMPOSITION`: when the input method committed text, it is typed; the composition itself is shown by the
/// system's own window for it.
fn committed(state: &WindowState, window: WindowHandle, message: Message) -> Option<LRESULT> {
    let flags = u32::try_from(message.lparam).unwrap_or_default();
    if flags & imm32::GCS_RESULTSTR == 0 {
        return None;
    }
    if let Some(text) = ime::committed_text(window).filter(|text| !text.is_empty()) {
        state.push(InputEvent::TextTyped(text));
    }
    // Handled here, so that the system does not also send the committed text again as WM_CHAR.
    Some(0)
}

/// The modifier keys held now, as the thread's view of the keyboard says, which follows the messages taken from the
/// queue so far.
pub(crate) fn modifiers_held() -> Modifiers {
    let mut held = Modifiers::NONE;
    let keys = [
        (user32::VK_SHIFT, Modifiers::SHIFT),
        (user32::VK_CONTROL, Modifiers::CONTROL),
        (user32::VK_MENU, Modifiers::ALT),
        (user32::VK_LWIN, Modifiers::META),
        (user32::VK_RWIN, Modifiers::META),
    ];
    for (virtual_key, modifier) in keys {
        // SAFETY: only an integer is passed.
        if unsafe { user32::GetKeyState(i32::from(virtual_key)) } < 0 {
            held |= modifier;
        }
    }
    held
}
