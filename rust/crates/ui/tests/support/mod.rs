//! What the tests of `ui` share: the fonts that come with the program, a window without a window that draws frames
//! from synthetic input, and the comparison of a frame with its reference image.

#![allow(dead_code, reason = "each test file uses its own part of the support")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a helper of a test that cannot go on fails the test"
)]

use std::path::Path;
use std::time::Duration;

use render::{Appearance, Assets, FontFamily, FontWeight, FrameBuffer, Point, Theme, ThemeName};
use testkit::png::{self, Image};
use ui::{
    FrameInput, FrameOutput, FrameSettings, InputEvent, Key, KeyEvent, Modifiers, NamedKey, PointerButton,
    PointerButtonEvent, PointerMove, Typefaces, Ui, UiMemory, WheelDelta, WheelEvent,
};

/// Assets with the fonts that come with the program, and no font of the system, and their typefaces.
pub fn assets() -> (Assets, Typefaces) {
    let mut assets = Assets::bundled();
    let mut add = |family, weight| assets.fonts.add_typeface(&family, weight).unwrap();
    let typefaces = Typefaces {
        sans: add(FontFamily::Interface, FontWeight::REGULAR),
        medium: add(FontFamily::Interface, FontWeight::MEDIUM),
        bold: add(FontFamily::Interface, FontWeight::SEMIBOLD),
        mono: add(FontFamily::Code, FontWeight::REGULAR),
    };
    (assets, typefaces)
}

/// A window drawn in memory: its frame buffer, fonts, the memory of the toolkit, the theme, the scale and the time.
pub struct Window {
    /// The pixels.
    pub frame: FrameBuffer,
    /// The fonts.
    pub assets: Assets,
    /// What the toolkit keeps between frames.
    pub memory: UiMemory,
    /// The typefaces.
    pub typefaces: Typefaces,
    /// The theme.
    pub theme: Theme,
    /// The scale.
    pub scale: f32,
    /// The time of the next frame.
    pub now: Duration,
}

impl Window {
    /// A window of a size in logical pixels, at a scale, in a theme, at time zero.
    pub fn new(theme: ThemeName, (width, height): (u32, u32), scale: u8) -> Window {
        let (assets, typefaces) = assets();
        Window {
            frame: FrameBuffer::new(width * u32::from(scale), height * u32::from(scale)),
            assets,
            memory: UiMemory::new(),
            typefaces,
            theme: Theme::new(theme, Appearance::Dark),
            scale: f32::from(scale),
            now: Duration::ZERO,
        }
    }

    /// Draws a frame of a view with some input, and returns what the view returned and what the frame asks for.
    pub fn frame<R>(&mut self, events: &[InputEvent], view: impl FnOnce(&mut Ui<'_>) -> R) -> (R, FrameOutput) {
        let mut ui = Ui::new(FrameInput {
            frame: &mut self.frame,
            assets: &mut self.assets,
            memory: &mut self.memory,
            events,
            settings: FrameSettings {
                theme: self.theme,
                typefaces: self.typefaces,
                scale: self.scale,
                now: self.now,
                file_icons: ui::FileIconStyle::default(),
            },
        });
        let result = view(&mut ui);
        let output = ui.finish();
        (result, output)
    }

    /// Moves the time on.
    pub fn wait(&mut self, milliseconds: u64) {
        self.now += Duration::from_millis(milliseconds);
    }

    /// Checks the pixels against `tests/snapshots/<name>.png`.
    pub fn assert_snapshot(&self, name: &str) {
        let reference = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/snapshots/{name}.png"));
        let actual = Image {
            width: self.frame.width(),
            height: self.frame.height(),
            pixels: self.frame.to_rgba(),
        };
        png::assert_snapshot(reference, &actual);
    }
}

/// The pointer moved to a point.
pub fn move_to(x: f32, y: f32) -> InputEvent {
    InputEvent::PointerMoved(PointerMove {
        position: Point::new(x, y),
        modifiers: Modifiers::NONE,
    })
}

/// A button event at a point.
fn button(x: f32, y: f32, modifiers: Modifiers, click_count: u32) -> PointerButtonEvent {
    PointerButtonEvent {
        button: PointerButton::Primary,
        position: Point::new(x, y),
        modifiers,
        click_count,
    }
}

/// The main button pressed at a point.
pub fn press(x: f32, y: f32) -> InputEvent {
    InputEvent::PointerPressed(button(x, y, Modifiers::NONE, 1))
}

/// The main button released at a point.
pub fn release(x: f32, y: f32) -> InputEvent {
    InputEvent::PointerReleased(button(x, y, Modifiers::NONE, 1))
}

/// A click of the main button at a point, with modifiers held and a count of clicks.
pub fn click_with(x: f32, y: f32, modifiers: Modifiers, click_count: u32) -> Vec<InputEvent> {
    vec![
        move_to(x, y),
        InputEvent::PointerPressed(button(x, y, modifiers, click_count)),
        InputEvent::PointerReleased(button(x, y, modifiers, click_count)),
    ]
}

/// A click of the main button at a point.
pub fn click(x: f32, y: f32) -> Vec<InputEvent> {
    click_with(x, y, Modifiers::NONE, 1)
}

/// A key pressed with modifiers.
pub fn key(named: NamedKey, modifiers: Modifiers) -> InputEvent {
    InputEvent::KeyPressed(KeyEvent::press(Key::Named(named), modifiers))
}

/// A key that types a character pressed with modifiers, such as Ctrl+A.
pub fn shortcut(character: char, modifiers: Modifiers) -> InputEvent {
    InputEvent::KeyPressed(KeyEvent::press(Key::Character(character), modifiers))
}

/// Text typed.
pub fn typed(text: &str) -> InputEvent {
    InputEvent::TextTyped(text.to_owned())
}

/// A turn of the wheel by lines down at a point.
pub fn wheel(x: f32, y: f32, lines: f32) -> InputEvent {
    InputEvent::Wheel(WheelEvent {
        delta: WheelDelta::Lines { x: 0.0, y: lines },
        position: Point::new(x, y),
        modifiers: Modifiers::NONE,
    })
}
