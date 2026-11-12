//! The shape of the pointer over a window, from the cursors every Windows has.

use super::procedure::{Message, WindowState};
use super::types::LRESULT;
use super::{low_word, user32};
use crate::event::Cursor;

/// Shows a cursor now.
pub(crate) fn show(cursor: Cursor) {
    let name = match cursor {
        Cursor::Arrow => user32::IDC_ARROW,
        Cursor::Text => user32::IDC_IBEAM,
        Cursor::Hand => user32::IDC_HAND,
        Cursor::ResizeEastWest => user32::IDC_SIZEWE,
        Cursor::ResizeNorthSouth => user32::IDC_SIZENS,
        // Windows has no hand that grabs; the four arrows of moving say the same.
        Cursor::Grab => user32::IDC_SIZEALL,
        Cursor::NotAllowed => user32::IDC_NO,
    };
    // A cursor of the system is named by a number in place of a string pointer, as MAKEINTRESOURCE does.
    let name = std::ptr::without_provenance::<u16>(usize::from(name));
    // SAFETY: with no module, the name is one of the system's cursors, which are shared and need no release.
    let handle = unsafe { user32::LoadCursorW(0, name) };
    if handle != 0 {
        // SAFETY: the handle is a cursor just loaded. The result is the cursor shown before, which is not needed.
        let _ = unsafe { user32::SetCursor(handle) };
    }
}

/// `WM_SETCURSOR`: over the inside of the window, the window's cursor; over its frame, the system's.
pub(crate) fn cursor_message(state: &WindowState, message: Message) -> Option<LRESULT> {
    if low_word(message.lparam.cast_unsigned()) != user32::HTCLIENT {
        return None;
    }
    show(state.cursor.get());
    Some(1)
}
