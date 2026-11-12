//! What the event loop hands to the program, and the names it gives to windows and timers.

use crate::input::InputEvent;

/// Something the event loop hands to the program's handler.
#[derive(Clone, Debug, PartialEq)]
pub enum Event<M> {
    /// Input for a window, or a change to it.
    Input {
        /// The window.
        window: WindowId,
        /// What happened to it.
        event: InputEvent,
    },
    /// A message another thread sent through a waker.
    Message(M),
    /// A timer went off. It goes off again after its interval until it is stopped.
    Timer(TimerId),
    /// The system changed between dark and light, or turned high contrast on or off.
    ThemeChanged,
}

/// The name of a window, unique in its event loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(pub(crate) u64);

/// The name of a timer, chosen by the program.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TimerId(pub u32);

/// The shape of the pointer over a window.
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
