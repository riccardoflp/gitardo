//! The input of the platform as the toolkit takes it, and the shape of the pointer the toolkit asks for as the platform
//! shows it. The two crates have types of their own (`ui` may not use `platform`), alike field for field.

use platform::InputEvent as SystemEvent;
use ui::InputEvent as ToolkitEvent;

/// The event of the toolkit for an event of the platform, or nothing for what is about the window rather than its
/// content: its size, its scale, a request to close it or to draw it, or what a screen reader asks of it.
#[must_use]
pub fn toolkit_event(event: &SystemEvent) -> Option<ToolkitEvent> {
    let translated = match event {
        SystemEvent::PointerMoved(moved) => ToolkitEvent::PointerMoved(ui::PointerMove {
            position: point(moved.position),
            modifiers: modifiers(moved.modifiers),
        }),
        SystemEvent::PointerPressed(pressed) => ToolkitEvent::PointerPressed(button_event(*pressed)),
        SystemEvent::PointerReleased(released) => ToolkitEvent::PointerReleased(button_event(*released)),
        SystemEvent::Wheel(wheel) => ToolkitEvent::Wheel(ui::WheelEvent {
            delta: wheel_delta(wheel.delta),
            position: point(wheel.position),
            modifiers: modifiers(wheel.modifiers),
        }),
        SystemEvent::PointerLeft => ToolkitEvent::PointerLeft,
        SystemEvent::KeyPressed(key) => ToolkitEvent::KeyPressed(key_event(*key)),
        SystemEvent::KeyReleased(key) => ToolkitEvent::KeyReleased(key_event(*key)),
        SystemEvent::TextTyped(text) => ToolkitEvent::TextTyped(text.clone()),
        SystemEvent::FocusGained => ToolkitEvent::FocusGained,
        SystemEvent::FocusLost => ToolkitEvent::FocusLost,
        SystemEvent::MoveStarted => ToolkitEvent::WindowMoveStarted,
        // The text the input method composes is not drawn by the toolkit yet: only what it types reaches it.
        SystemEvent::TextComposing(_)
        | SystemEvent::Resized(_)
        | SystemEvent::ScaleChanged(_)
        | SystemEvent::CloseRequested
        | SystemEvent::RedrawRequested
        | SystemEvent::Accessibility(_) => {
            return None;
        }
    };
    Some(translated)
}

/// Whether a key pastes: Ctrl+V, Ctrl+Shift+V as in a terminal, or Shift+Insert. The application reads the clipboard
/// then and hands its text to the toolkit, which cannot read it.
#[must_use]
pub fn is_paste(key: &platform::KeyEvent) -> bool {
    let control = platform::Modifiers::CONTROL;
    let control_shift = platform::Modifiers::CONTROL | platform::Modifiers::SHIFT;
    let control_v =
        key.key == platform::Key::Character('v') && (key.modifiers == control || key.modifiers == control_shift);
    let shift_insert =
        key.key == platform::Key::Named(platform::NamedKey::Insert) && key.modifiers == platform::Modifiers::SHIFT;
    control_v || shift_insert
}

/// The shape of the pointer the platform shows for the one the toolkit asks for.
#[must_use]
pub fn system_cursor(cursor: ui::Cursor) -> platform::Cursor {
    match cursor {
        ui::Cursor::Arrow => platform::Cursor::Arrow,
        ui::Cursor::Text => platform::Cursor::Text,
        ui::Cursor::Hand => platform::Cursor::Hand,
        ui::Cursor::ResizeEastWest => platform::Cursor::ResizeEastWest,
        ui::Cursor::ResizeNorthSouth => platform::Cursor::ResizeNorthSouth,
        ui::Cursor::Grab => platform::Cursor::Grab,
        ui::Cursor::NotAllowed => platform::Cursor::NotAllowed,
    }
}

/// A point in logical pixels.
fn point(point: platform::Point) -> render::Point {
    render::Point::new(point.x, point.y)
}

/// The modifier keys held.
fn modifiers(held: platform::Modifiers) -> ui::Modifiers {
    let pairs = [
        (platform::Modifiers::SHIFT, ui::Modifiers::SHIFT),
        (platform::Modifiers::CONTROL, ui::Modifiers::CONTROL),
        (platform::Modifiers::ALT, ui::Modifiers::ALT),
        (platform::Modifiers::META, ui::Modifiers::META),
    ];
    pairs
        .into_iter()
        .filter(|(system, _)| held.contains(*system))
        .fold(ui::Modifiers::NONE, |all, (_, toolkit)| all | toolkit)
}

/// A button pressed or released.
fn button_event(event: platform::PointerButtonEvent) -> ui::PointerButtonEvent {
    ui::PointerButtonEvent {
        button: button(event.button),
        position: point(event.position),
        modifiers: modifiers(event.modifiers),
        click_count: event.click_count,
    }
}

/// A button of the pointer.
fn button(button: platform::PointerButton) -> ui::PointerButton {
    match button {
        platform::PointerButton::Primary => ui::PointerButton::Primary,
        platform::PointerButton::Secondary => ui::PointerButton::Secondary,
        platform::PointerButton::Middle => ui::PointerButton::Middle,
        platform::PointerButton::Back => ui::PointerButton::Back,
        platform::PointerButton::Forward => ui::PointerButton::Forward,
    }
}

/// How far the wheel scrolls.
fn wheel_delta(delta: platform::WheelDelta) -> ui::WheelDelta {
    match delta {
        platform::WheelDelta::Lines { x, y } => ui::WheelDelta::Lines { x, y },
        platform::WheelDelta::Pixels { x, y } => ui::WheelDelta::Pixels { x, y },
    }
}

/// A key pressed or released.
fn key_event(event: platform::KeyEvent) -> ui::KeyEvent {
    ui::KeyEvent {
        key: key(event.key),
        modifiers: modifiers(event.modifiers),
        is_repeat: event.is_repeat,
    }
}

/// A key.
fn key(key: platform::Key) -> ui::Key {
    match key {
        platform::Key::Character(character) => ui::Key::Character(character),
        platform::Key::Named(named) => ui::Key::Named(named_key(named)),
    }
}

/// A key that does not type.
fn named_key(key: platform::NamedKey) -> ui::NamedKey {
    use platform::NamedKey as System;
    use ui::NamedKey as Toolkit;
    match key {
        System::Enter => Toolkit::Enter,
        System::Tab => Toolkit::Tab,
        System::Escape => Toolkit::Escape,
        System::Backspace => Toolkit::Backspace,
        System::Delete => Toolkit::Delete,
        System::Insert => Toolkit::Insert,
        System::Home => Toolkit::Home,
        System::End => Toolkit::End,
        System::PageUp => Toolkit::PageUp,
        System::PageDown => Toolkit::PageDown,
        System::ArrowLeft => Toolkit::ArrowLeft,
        System::ArrowRight => Toolkit::ArrowRight,
        System::ArrowUp => Toolkit::ArrowUp,
        System::ArrowDown => Toolkit::ArrowDown,
        System::Function(number) => Toolkit::Function(number),
        System::Shift => Toolkit::Shift,
        System::Control => Toolkit::Control,
        System::Alt => Toolkit::Alt,
        System::Meta => Toolkit::Meta,
        System::CapsLock => Toolkit::CapsLock,
        System::NumLock => Toolkit::NumLock,
        System::ScrollLock => Toolkit::ScrollLock,
        System::PrintScreen => Toolkit::PrintScreen,
        System::Pause => Toolkit::Pause,
        System::ContextMenu => Toolkit::ContextMenu,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_keeps_its_modifiers_and_whether_it_repeats() {
        let pressed = SystemEvent::KeyPressed(platform::KeyEvent {
            key: platform::Key::Character('p'),
            modifiers: platform::Modifiers::CONTROL | platform::Modifiers::SHIFT,
            is_repeat: true,
        });
        let expected = ToolkitEvent::KeyPressed(ui::KeyEvent {
            key: ui::Key::Character('p'),
            modifiers: ui::Modifiers::CONTROL | ui::Modifiers::SHIFT,
            is_repeat: true,
        });
        assert_eq!(toolkit_event(&pressed), Some(expected));
    }

    #[test]
    fn a_click_and_the_wheel_keep_their_place() {
        let at = platform::Point { x: 12.5, y: 40.0 };
        let pressed = SystemEvent::PointerPressed(platform::PointerButtonEvent {
            button: platform::PointerButton::Secondary,
            position: at,
            modifiers: platform::Modifiers::NONE,
            click_count: 2,
        });
        let expected = ToolkitEvent::PointerPressed(ui::PointerButtonEvent {
            button: ui::PointerButton::Secondary,
            position: render::Point::new(12.5, 40.0),
            modifiers: ui::Modifiers::NONE,
            click_count: 2,
        });
        assert_eq!(toolkit_event(&pressed), Some(expected));
        let wheel = SystemEvent::Wheel(platform::WheelEvent {
            delta: platform::WheelDelta::Lines { x: 0.0, y: 3.0 },
            position: at,
            modifiers: platform::Modifiers::ALT,
        });
        let Some(ToolkitEvent::Wheel(turned)) = toolkit_event(&wheel) else {
            panic!("the wheel is the wheel");
        };
        assert_eq!(turned.delta, ui::WheelDelta::Lines { x: 0.0, y: 3.0 });
        assert_eq!(turned.modifiers, ui::Modifiers::ALT);
    }

    #[test]
    fn what_is_about_the_window_is_not_for_the_toolkit() {
        assert_eq!(toolkit_event(&SystemEvent::CloseRequested), None);
        assert_eq!(toolkit_event(&SystemEvent::RedrawRequested), None);
        let text = SystemEvent::TextTyped(String::from("è"));
        assert_eq!(toolkit_event(&text), Some(ToolkitEvent::TextTyped(String::from("è"))));
    }

    #[test]
    fn ctrl_v_and_shift_insert_paste() {
        let key = |key, modifiers| platform::KeyEvent {
            key,
            modifiers,
            is_repeat: false,
        };
        let insert = platform::Key::Named(platform::NamedKey::Insert);
        assert!(is_paste(&key(
            platform::Key::Character('v'),
            platform::Modifiers::CONTROL
        )));
        assert!(is_paste(&key(insert, platform::Modifiers::SHIFT)));
        assert!(!is_paste(&key(
            platform::Key::Character('v'),
            platform::Modifiers::NONE
        )));
        assert!(!is_paste(&key(insert, platform::Modifiers::CONTROL)));
    }
}
