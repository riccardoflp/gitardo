//! The pointer messages of a window: moves, buttons, the wheel, and leaving. A window keeps the pointer while a
//! button is held, so that a drag that leaves it still reaches it.

use core::ffi::c_void;

use super::handle::WindowHandle;
use super::keyboard::modifiers_held;
use super::procedure::{Message, WindowState};
use super::types::{LRESULT, POINT};
use super::{coordinates, high_word, user32};
use crate::click::{ClickCounter, ClickLimits, Press};
use crate::convert::to_float;
use crate::input::{InputEvent, PointerButton, PointerButtonEvent, PointerMove, WheelDelta, WheelEvent};

/// What the setting of the wheel says when a notch should scroll a page; Gitardo scrolls this many lines instead.
const LINES_FOR_A_PAGE: u32 = 20;

/// What a window knows of the pointer between messages.
#[derive(Debug, Default)]
pub(crate) struct PointerState {
    held: u8,
    is_tracking_leave: bool,
    clicks: ClickCounter,
}

/// Whether a button went down or up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Down,
    Up,
}

/// What a pointer message does; `None` for the messages the system handles alone.
pub(crate) fn pointer_message(state: &WindowState, window: WindowHandle, message: Message) -> Option<LRESULT> {
    if let Some((button, direction)) = button_message(message) {
        match direction {
            Direction::Down => pressed(state, window, message, button),
            Direction::Up => released(state, message, button),
        }
        // The side buttons are answered with TRUE, as their documentation asks.
        return Some(LRESULT::from(matches!(
            button,
            PointerButton::Back | PointerButton::Forward
        )));
    }
    match message.id {
        user32::WM_MOUSEMOVE => moved(state, window, message),
        user32::WM_MOUSEWHEEL | user32::WM_MOUSEHWHEEL => wheel(state, window, message),
        user32::WM_MOUSELEAVE => {
            state.pointer.borrow_mut().is_tracking_leave = false;
            state.push(InputEvent::PointerLeft);
        }
        user32::WM_CAPTURECHANGED => state.pointer.borrow_mut().held = 0,
        _ => return None,
    }
    Some(0)
}

/// The button of a button message, and whether it went down or up.
fn button_message(message: Message) -> Option<(PointerButton, Direction)> {
    let side = || {
        if high_word(message.wparam) == user32::XBUTTON1 {
            PointerButton::Back
        } else {
            PointerButton::Forward
        }
    };
    let found = match message.id {
        user32::WM_LBUTTONDOWN => (PointerButton::Primary, Direction::Down),
        user32::WM_LBUTTONUP => (PointerButton::Primary, Direction::Up),
        user32::WM_RBUTTONDOWN => (PointerButton::Secondary, Direction::Down),
        user32::WM_RBUTTONUP => (PointerButton::Secondary, Direction::Up),
        user32::WM_MBUTTONDOWN => (PointerButton::Middle, Direction::Down),
        user32::WM_MBUTTONUP => (PointerButton::Middle, Direction::Up),
        user32::WM_XBUTTONDOWN => (side(), Direction::Down),
        user32::WM_XBUTTONUP => (side(), Direction::Up),
        _ => return None,
    };
    Some(found)
}

/// The bit of a button in the set of buttons held.
fn button_bit(button: PointerButton) -> u8 {
    match button {
        PointerButton::Primary => 1,
        PointerButton::Secondary => 2,
        PointerButton::Middle => 4,
        PointerButton::Back => 8,
        PointerButton::Forward => 16,
    }
}

/// A button went down: it is counted as a click, and the window keeps the pointer until every button is up.
fn pressed(state: &WindowState, window: WindowHandle, message: Message, button: PointerButton) {
    let (x, y) = coordinates(message.lparam);
    // SAFETY: the call takes nothing and reads the time of the message being handled.
    let time = unsafe { user32::GetMessageTime() }.cast_unsigned();
    let press = Press {
        button,
        x: i32::from(x),
        y: i32::from(y),
        time,
    };
    let limits = click_limits(state.dpi.get());
    let (click_count, was_idle) = {
        let mut pointer = state.pointer.borrow_mut();
        let was_idle = pointer.held == 0;
        pointer.held |= button_bit(button);
        (pointer.clicks.press(press, limits), was_idle)
    };
    if was_idle {
        // SAFETY: the window belongs to this thread. The result is the window that had the pointer before.
        let _ = unsafe { user32::SetCapture(window.raw()) };
    }
    let position = state.scale().to_logical_point(x, y);
    let event = PointerButtonEvent {
        button,
        position,
        modifiers: modifiers_held(),
        click_count,
    };
    state.push(InputEvent::PointerPressed(event));
}

/// A button went up: once every button is up, the window lets the pointer go.
fn released(state: &WindowState, message: Message, button: PointerButton) {
    let (x, y) = coordinates(message.lparam);
    let (click_count, is_idle) = {
        let mut pointer = state.pointer.borrow_mut();
        pointer.held &= !button_bit(button);
        (pointer.clicks.last_count(), pointer.held == 0)
    };
    if is_idle {
        // SAFETY: the call takes nothing. It fails only when no window of the thread has the pointer, which is the
        // state wanted.
        let _ = unsafe { user32::ReleaseCapture() };
    }
    let position = state.scale().to_logical_point(x, y);
    let event = PointerButtonEvent {
        button,
        position,
        modifiers: modifiers_held(),
        click_count,
    };
    state.push(InputEvent::PointerReleased(event));
}

/// The pointer moved: the first move after it entered asks the system to say when it leaves.
fn moved(state: &WindowState, window: WindowHandle, message: Message) {
    let is_tracking = std::mem::replace(&mut state.pointer.borrow_mut().is_tracking_leave, true);
    if !is_tracking {
        track_leave(window);
    }
    let (x, y) = coordinates(message.lparam);
    let position = state.scale().to_logical_point(x, y);
    state.push(InputEvent::PointerMoved(PointerMove {
        position,
        modifiers: modifiers_held(),
    }));
}

/// Asks the system for `WM_MOUSELEAVE` when the pointer leaves the window.
fn track_leave(window: WindowHandle) {
    let mut track = user32::TRACKMOUSEEVENT {
        cbSize: u32::try_from(size_of::<user32::TRACKMOUSEEVENT>()).unwrap_or_default(),
        dwFlags: user32::TME_LEAVE,
        hwndTrack: window.raw(),
        dwHoverTime: 0,
    };
    // SAFETY: `track` is filled in as TME_LEAVE asks and alive for the call. When the system refuses, the window
    // only misses the next leave, which the next move asks for again.
    let _ = unsafe { user32::TrackMouseEvent(&raw mut track) };
}

/// The wheel turned: down or across, in lines as the system's setting says.
fn wheel(state: &WindowState, window: WindowHandle, message: Message) {
    let notches = high_word(message.wparam).cast_signed();
    let delta = if message.id == user32::WM_MOUSEWHEEL {
        WheelDelta::Lines {
            x: 0.0,
            y: -wheel_lines(notches, wheel_setting(user32::SPI_GETWHEELSCROLLLINES)),
        }
    } else {
        WheelDelta::Lines {
            x: wheel_lines(notches, wheel_setting(user32::SPI_GETWHEELSCROLLCHARS)),
            y: 0.0,
        }
    };
    let (x, y) = coordinates(message.lparam);
    let mut point = POINT {
        x: i32::from(x),
        y: i32::from(y),
    };
    // SAFETY: `point` is a POINT the call reads and writes, alive for the call. When it fails, the point stays on the
    // screen's coordinates, which only misplaces this one turn of the wheel.
    let _ = unsafe { user32::ScreenToClient(window.raw(), &raw mut point) };
    let inside = |value: i32| i16::try_from(value).unwrap_or(if value < 0 { i16::MIN } else { i16::MAX });
    let position = state.scale().to_logical_point(inside(point.x), inside(point.y));
    state.push(InputEvent::Wheel(WheelEvent {
        delta,
        position,
        modifiers: modifiers_held(),
    }));
}

/// The lines a turn of the wheel scrolls: a notch is 120, and scrolls as many lines as the setting says.
fn wheel_lines(delta: i16, lines_per_notch: u32) -> f32 {
    let lines = if lines_per_notch == u32::MAX {
        LINES_FOR_A_PAGE
    } else {
        lines_per_notch
    };
    f32::from(delta) / f32::from(user32::WHEEL_DELTA) * to_float(lines)
}

/// A setting of the wheel: how many lines, or characters across, a notch scrolls; 3 when it cannot be read.
fn wheel_setting(action: u32) -> u32 {
    let mut lines: u32 = 3;
    // SAFETY: both settings write a u32 to the pointer, which points to one alive for the call. When the call fails,
    // the value stays 3, the system's own default.
    let _ = unsafe { user32::SystemParametersInfoW(action, 0, (&raw mut lines).cast::<c_void>(), 0) };
    lines
}

/// How soon and how near the presses of a double click must be, as the person set it.
fn click_limits(dpi: u32) -> ClickLimits {
    // SAFETY: the calls take integers only.
    let (time, width, height) = unsafe {
        (
            user32::GetDoubleClickTime(),
            user32::GetSystemMetricsForDpi(user32::SM_CXDOUBLECLK, dpi),
            user32::GetSystemMetricsForDpi(user32::SM_CYDOUBLECLK, dpi),
        )
    };
    ClickLimits { time, width, height }
}

#[cfg(test)]
mod tests {
    use super::{Direction, Message, button_message, wheel_lines};
    use crate::input::PointerButton;
    use crate::windows::user32;

    #[test]
    fn scrolls_lines_by_notches() {
        assert!((wheel_lines(120, 3) - 3.0).abs() < f32::EPSILON);
        assert!((wheel_lines(-240, 3) + 6.0).abs() < f32::EPSILON);
        assert!((wheel_lines(60, 3) - 1.5).abs() < f32::EPSILON);
        assert!((wheel_lines(120, u32::MAX) - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn knows_the_buttons_of_messages() {
        let message = |id, wparam| Message { id, wparam, lparam: 0 };
        let cases = [
            (
                message(user32::WM_LBUTTONDOWN, 0),
                Some((PointerButton::Primary, Direction::Down)),
            ),
            (
                message(user32::WM_RBUTTONUP, 0),
                Some((PointerButton::Secondary, Direction::Up)),
            ),
            (
                message(user32::WM_XBUTTONDOWN, 0x0001_0000),
                Some((PointerButton::Back, Direction::Down)),
            ),
            (
                message(user32::WM_XBUTTONUP, 0x0002_0000),
                Some((PointerButton::Forward, Direction::Up)),
            ),
            (message(user32::WM_MOUSEMOVE, 0), None),
        ];
        for (given, expected) in cases {
            assert_eq!(button_message(given), expected, "{given:?}");
        }
    }
}
