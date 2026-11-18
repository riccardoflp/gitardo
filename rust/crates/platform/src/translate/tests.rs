use winit::dpi::PhysicalPosition;
use winit::event::{MouseButton, MouseScrollDelta};
use winit::keyboard::{Key as LibraryKey, ModifiersState, NamedKey as LibraryNamedKey, NativeKey, SmolStr};
use winit::window::{CursorIcon, Theme};

use super::{
    appearance, button, cursor_icon, key, library_theme, logical_point, modifier_of, modifiers, typed_text, types_text,
    wheel_delta,
};
use crate::event::Cursor;
use crate::geometry::{Point, Scale};
use crate::input::{Key, Modifiers, NamedKey, PointerButton, WheelDelta};
use crate::system::WheelSteps;
use crate::theme::Appearance;

fn character(text: &str) -> LibraryKey {
    LibraryKey::Character(SmolStr::new(text))
}

#[test]
fn names_keys_by_what_they_mean() {
    let cases = [
        (character("a"), Some(Key::Character('a'))),
        (character("A"), Some(Key::Character('a'))),
        (character("è"), Some(Key::Character('è'))),
        (character(","), Some(Key::Character(','))),
        (character("ab"), None),
        (LibraryKey::Named(LibraryNamedKey::Space), Some(Key::Character(' '))),
        (
            LibraryKey::Named(LibraryNamedKey::Enter),
            Some(Key::Named(NamedKey::Enter)),
        ),
        (
            LibraryKey::Named(LibraryNamedKey::ArrowDown),
            Some(Key::Named(NamedKey::ArrowDown)),
        ),
        (
            LibraryKey::Named(LibraryNamedKey::AltGraph),
            Some(Key::Named(NamedKey::Alt)),
        ),
        (
            LibraryKey::Named(LibraryNamedKey::Super),
            Some(Key::Named(NamedKey::Meta)),
        ),
        (
            LibraryKey::Named(LibraryNamedKey::F1),
            Some(Key::Named(NamedKey::Function(1))),
        ),
        (
            LibraryKey::Named(LibraryNamedKey::F24),
            Some(Key::Named(NamedKey::Function(24))),
        ),
        (LibraryKey::Named(LibraryNamedKey::F25), None),
        (LibraryKey::Named(LibraryNamedKey::MediaPlay), None),
        (LibraryKey::Dead(Some('^')), None),
        (LibraryKey::Unidentified(NativeKey::Unidentified), None),
    ];
    for (given, expected) in cases {
        assert_eq!(key(&given), expected, "{given:?}");
    }
}

#[test]
fn knows_which_keys_are_modifiers() {
    assert_eq!(modifier_of(Key::Named(NamedKey::Shift)), Modifiers::SHIFT);
    assert_eq!(modifier_of(Key::Named(NamedKey::Meta)), Modifiers::META);
    assert_eq!(modifier_of(Key::Named(NamedKey::Enter)), Modifiers::NONE);
    assert_eq!(modifier_of(Key::Character('s')), Modifiers::NONE);
    assert_eq!(modifiers(ModifiersState::empty()), Modifiers::NONE);
    assert_eq!(
        modifiers(ModifiersState::CONTROL | ModifiersState::SHIFT),
        Modifiers::CONTROL | Modifiers::SHIFT
    );
    assert_eq!(
        modifiers(ModifiersState::ALT | ModifiersState::SUPER),
        Modifiers::ALT | Modifiers::META
    );
}

#[test]
fn keeps_only_the_text_of_typed_text() {
    let cases = [
        ("a", Some("a")),
        ("é😀", Some("é😀")),
        ("@", Some("@")),
        ("\r", None),
        ("\t", None),
        ("\u{8}", None),
        ("\u{16}", None),
        ("a\u{1b}b", Some("ab")),
        ("", None),
    ];
    for (given, expected) in cases {
        assert_eq!(typed_text(given).as_deref(), expected, "{given:?}");
    }
}

#[test]
fn types_no_text_with_alt_alone_and_on_macos_with_ctrl_and_option() {
    let on_macos = cfg!(target_os = "macos");
    let cases = [
        (Modifiers::NONE, true),
        (Modifiers::SHIFT, true),
        (Modifiers::ALT, on_macos),
        (Modifiers::ALT | Modifiers::SHIFT, on_macos),
        (Modifiers::ALT | Modifiers::CONTROL, !on_macos),
    ];
    for (held, expected) in cases {
        assert_eq!(types_text(held), expected, "{held:?}");
    }
}

#[test]
fn turns_buttons_cursors_and_themes() {
    assert_eq!(button(MouseButton::Left), Some(PointerButton::Primary));
    assert_eq!(button(MouseButton::Right), Some(PointerButton::Secondary));
    assert_eq!(button(MouseButton::Back), Some(PointerButton::Back));
    assert_eq!(button(MouseButton::Other(9)), None);
    assert_eq!(cursor_icon(Cursor::Text), CursorIcon::Text);
    assert_eq!(cursor_icon(Cursor::Hand), CursorIcon::Pointer);
    assert_eq!(cursor_icon(Cursor::ResizeEastWest), CursorIcon::EwResize);
    for given in [Appearance::Light, Appearance::Dark] {
        assert_eq!(appearance(library_theme(given)), given);
    }
    assert_eq!(appearance(Theme::Dark), Appearance::Dark);
}

#[test]
fn scrolls_the_way_the_view_goes() {
    let steps = WheelSteps { down: 3.0, across: 2.0 };
    let notch_down = MouseScrollDelta::LineDelta(0.0, -1.0);
    assert_eq!(
        wheel_delta(notch_down, steps, Scale::default()),
        WheelDelta::Lines { x: 0.0, y: 3.0 }
    );
    let notch_left = MouseScrollDelta::LineDelta(1.0, 0.0);
    assert_eq!(
        wheel_delta(notch_left, steps, Scale::default()),
        WheelDelta::Lines { x: -2.0, y: 0.0 }
    );
    let touchpad = MouseScrollDelta::PixelDelta(PhysicalPosition::new(10.0, -40.0));
    assert_eq!(
        wheel_delta(touchpad, steps, Scale::from_dpi(192)),
        WheelDelta::Pixels { x: -5.0, y: 20.0 }
    );
    assert_eq!(
        logical_point(150.0, -30.0, Scale::from_dpi(144)),
        Point { x: 100.0, y: -20.0 }
    );
}
