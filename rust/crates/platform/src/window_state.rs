//! What a window keeps between the events of `winit`: the window itself, its pixels, its screen reader adapter, the
//! input that other threads hand it, and what the pointer and the keyboard were doing; and how it turns the events of
//! `winit` into its input.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use accesskit_winit::Adapter;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Ime, KeyEvent as LibraryKeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::keyboard::Key as LibraryKey;
use winit::window::Window as LibraryWindow;

use crate::accessibility::Requests;
use crate::click::{ClickCounter, Press};
use crate::convert::round_to_coordinate;
use crate::error::PlatformError;
use crate::event::{Cursor, WindowId};
use crate::geometry::{PhysicalSize, Point, Scale};
use crate::input::{InputEvent, KeyEvent, Modifiers, PointerButtonEvent, PointerMove, WheelEvent};
use crate::mailbox::Mailbox;
use crate::present::Presenter;
use crate::system::{click_limits, usable, wheel_steps};
use crate::translate;

/// What a window keeps. The fields drop in order, the window itself last, after what was made for it.
pub(crate) struct WindowShared {
    /// Its name.
    pub(crate) id: WindowId,
    /// What tells the screen reader about it.
    pub(crate) adapter: RefCell<Adapter>,
    /// What shows its frames.
    pub(crate) presenter: RefCell<Presenter>,
    /// Input from other threads: synthetic input and the requests of a screen reader.
    pub(crate) inbox: Arc<Mailbox<InputEvent>>,
    /// Where the pointer is, in physical pixels.
    pointer: Cell<PhysicalPosition<f64>>,
    /// The modifier keys held.
    modifiers: Cell<Modifiers>,
    /// The presses that make double and triple clicks.
    clicks: RefCell<ClickCounter>,
    /// Whether it shows.
    pub(crate) is_visible: Cell<bool>,
    /// Whether the program asked for a frame that has not come yet.
    pub(crate) is_redraw_asked: Cell<bool>,
    /// The shape of the pointer over it.
    pub(crate) cursor: Cell<Cursor>,
    /// Whether the input method is on.
    pub(crate) is_composing_allowed: Cell<bool>,
    /// The window of `winit`.
    pub(crate) window: Rc<LibraryWindow>,
}

impl WindowShared {
    /// What a new window keeps, before it shows: the screen reader adapter must be made while it is hidden.
    pub(crate) fn new(
        active: &ActiveEventLoop,
        window: Rc<LibraryWindow>,
        proxy: EventLoopProxy<()>,
    ) -> Result<WindowShared, PlatformError> {
        let inbox = Arc::new(Mailbox::new(proxy));
        let adapter = Adapter::with_direct_handlers(
            active,
            &window,
            Requests::new(&inbox),
            Requests::new(&inbox),
            Requests::new(&inbox),
        );
        Ok(WindowShared {
            id: WindowId(u64::from(window.id())),
            adapter: RefCell::new(adapter),
            presenter: RefCell::new(Presenter::new(&window)?),
            inbox,
            pointer: Cell::new(PhysicalPosition::new(0.0, 0.0)),
            modifiers: Cell::new(Modifiers::NONE),
            clicks: RefCell::new(ClickCounter::default()),
            is_visible: Cell::new(false),
            is_redraw_asked: Cell::new(false),
            cursor: Cell::new(Cursor::Arrow),
            is_composing_allowed: Cell::new(false),
            window,
        })
    }

    /// The scale of the screen it is on.
    pub(crate) fn scale(&self) -> Scale {
        Scale::from_factor(self.window.scale_factor())
    }

    /// Lets the screen reader adapter see an event first, as it asks.
    pub(crate) fn show_to_adapter(&self, event: &WindowEvent) {
        self.adapter.borrow_mut().process_event(&self.window, event);
    }

    /// The input an event of `winit` makes; `clock` is when the loop was made, which the times of clicks count from.
    pub(crate) fn translate(&self, event: &WindowEvent, clock: Instant) -> Vec<InputEvent> {
        let translated = match event {
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => return self.keyboard(event),
            WindowEvent::Ime(Ime::Commit(text)) => translate::typed_text(text).map(InputEvent::TextTyped),
            WindowEvent::Ime(Ime::Preedit(text, _)) => Some(InputEvent::TextComposing(text.clone())),
            WindowEvent::ModifiersChanged(held) => {
                self.modifiers.set(translate::modifiers(held.state()));
                None
            }
            WindowEvent::CursorMoved { position, .. } => Some(self.moved(*position)),
            WindowEvent::CursorLeft { .. } => Some(InputEvent::PointerLeft),
            WindowEvent::MouseInput { state, button, .. } => self.button(*state, *button, clock),
            WindowEvent::MouseWheel { delta, .. } => Some(InputEvent::Wheel(WheelEvent {
                delta: translate::wheel_delta(*delta, wheel_steps(), self.scale()),
                position: self.position(),
                modifiers: self.modifiers.get(),
            })),
            WindowEvent::Focused(is_focused) => Some(if *is_focused {
                InputEvent::FocusGained
            } else {
                InputEvent::FocusLost
            }),
            WindowEvent::Resized(size) => Some(InputEvent::Resized(PhysicalSize {
                width: size.width,
                height: size.height,
            })),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                Some(InputEvent::ScaleChanged(Scale::from_factor(*scale_factor)))
            }
            WindowEvent::CloseRequested => Some(InputEvent::CloseRequested),
            WindowEvent::RedrawRequested => Some(InputEvent::RedrawRequested),
            // The rest is no input of Gitardo: the input method turned on or off, files hovered, touch, the theme
            // (which the loop reads itself), the window moved or destroyed, and the events `winit` makes up for keys
            // already held when the window gets the keyboard.
            _ => None,
        };
        translated.into_iter().collect()
    }

    /// A key pressed or released, and the text it typed.
    fn keyboard(&self, event: &LibraryKeyEvent) -> Vec<InputEvent> {
        let is_pressed = event.state == ElementState::Pressed;
        let mut events = Vec::new();
        if let Some(key) = translate::key(&unmodified_key(event)) {
            let key_event = KeyEvent {
                key,
                modifiers: self.modifiers.get() | translate::modifier_of(key),
                is_repeat: is_pressed && event.repeat,
            };
            events.push(if is_pressed {
                InputEvent::KeyPressed(key_event)
            } else {
                InputEvent::KeyReleased(key_event)
            });
        }
        if is_pressed && let Some(text) = typed(event).and_then(translate::typed_text) {
            events.push(InputEvent::TextTyped(text));
        }
        events
    }

    /// The pointer moved.
    fn moved(&self, position: PhysicalPosition<f64>) -> InputEvent {
        self.pointer.set(position);
        InputEvent::PointerMoved(PointerMove {
            position: self.position(),
            modifiers: self.modifiers.get(),
        })
    }

    /// A button pressed or released, with the count of clicks it makes.
    fn button(&self, state: ElementState, button: MouseButton, clock: Instant) -> Option<InputEvent> {
        let button = translate::button(button)?;
        let mut clicks = self.clicks.borrow_mut();
        let click_count = match state {
            ElementState::Pressed => {
                let physical = self.pointer.get();
                let press = Press {
                    button,
                    x: coordinate(physical.x),
                    y: coordinate(physical.y),
                    time: milliseconds_since(clock),
                };
                clicks.press(press, usable(click_limits(self.scale())))
            }
            ElementState::Released => clicks.last_count(),
        };
        let event = PointerButtonEvent {
            button,
            position: self.position(),
            modifiers: self.modifiers.get(),
            click_count,
        };
        Some(match state {
            ElementState::Pressed => InputEvent::PointerPressed(event),
            ElementState::Released => InputEvent::PointerReleased(event),
        })
    }

    /// Where the pointer is, in logical pixels.
    fn position(&self) -> Point {
        let physical = self.pointer.get();
        translate::logical_point(physical.x, physical.y, self.scale())
    }
}

/// The key as the layout gives it without modifiers: `a` for Shift+A and Ctrl+A alike.
#[cfg(not(any(
    target_os = "android",
    target_os = "ios",
    target_os = "redox",
    target_family = "wasm"
)))]
fn unmodified_key(event: &LibraryKeyEvent) -> LibraryKey {
    use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
    event.key_without_modifiers()
}

/// The key as the layout gives it, on the systems where `winit` cannot leave the modifiers out.
#[cfg(any(
    target_os = "android",
    target_os = "ios",
    target_os = "redox",
    target_family = "wasm"
))]
fn unmodified_key(event: &LibraryKeyEvent) -> LibraryKey {
    event.logical_key.clone()
}

/// The text a key typed with every modifier held, Ctrl and `AltGr` included: Ctrl+V types a control character, which
/// is left out, while `AltGr` types the character of the layout.
#[cfg(not(any(
    target_os = "android",
    target_os = "ios",
    target_os = "redox",
    target_family = "wasm"
)))]
fn typed(event: &LibraryKeyEvent) -> Option<&str> {
    use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
    event.text_with_all_modifiers()
}

/// The text a key typed, on the systems where `winit` gives no other.
#[cfg(any(
    target_os = "android",
    target_os = "ios",
    target_os = "redox",
    target_family = "wasm"
))]
fn typed(event: &LibraryKeyEvent) -> Option<&str> {
    event.text.as_deref()
}

/// A coordinate in physical pixels, rounded.
#[allow(
    clippy::cast_possible_truncation,
    reason = "positions on a screen are far inside the range of f32, and a cast of a float saturates"
)]
fn coordinate(value: f64) -> i32 {
    round_to_coordinate(value as f32)
}

/// The milliseconds since `clock`, on a clock of 32 bits that wraps around, as the counting of clicks takes them.
fn milliseconds_since(clock: Instant) -> u32 {
    let milliseconds = clock.elapsed().as_millis() % (u128::from(u32::MAX) + 1);
    u32::try_from(milliseconds).unwrap_or_default()
}
