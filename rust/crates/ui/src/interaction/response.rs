//! What happened to a widget in a frame, in the words a view asks about: was it clicked, is the pointer over it, does it
//! have the keyboard.

use render::Rect;

use super::hits::Sense;
use super::route::{Interaction, WidgetEvent};
use crate::id::WidgetId;
use crate::input::{Key, Modifiers, NamedKey, PointerButton};

/// What happened to a widget in a frame.
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each is a question a view asks of a widget, and any of them can be true together"
)]
pub struct Response {
    /// The widget.
    pub id: WidgetId,
    /// Where it is.
    pub rect: Rect,
    /// Whether the pointer is over it, and over nothing above it.
    pub hovered: bool,
    /// Whether the main button pressed on it is still held.
    pub pressed: bool,
    /// Whether it was clicked: pressed and released with the pointer over it, or Enter or Space pressed while it has
    /// the focus.
    pub clicked: bool,
    /// Whether the second click of a double click was released over it.
    pub double_clicked: bool,
    /// Whether the button of context menus was pressed and released over it, or the key of context menus pressed
    /// while it has the focus.
    pub secondary_clicked: bool,
    /// Whether the middle button was released over it after being pressed on it, as a click of the wheel.
    pub middle_clicked: bool,
    /// Whether it has the keyboard focus.
    pub has_focus: bool,
    /// Whether the focus is on it and shows a ring, because it came there by the keyboard.
    pub shows_focus: bool,
    /// Whether Escape was pressed while it has the focus.
    pub escaped: bool,
}

impl Response {
    /// What happened to a widget, from its events and what is remembered of the pointer and the focus.
    pub(crate) fn new(id: WidgetId, rect: Rect, sense: Sense, events: &[WidgetEvent], state: &Interaction) -> Response {
        let has_focus = state.focus == Some(id);
        let mut response = Response {
            id,
            rect,
            hovered: state.hovered == Some(id),
            pressed: state.active.is_some_and(|active| active.id == id),
            clicked: false,
            double_clicked: false,
            secondary_clicked: false,
            middle_clicked: false,
            has_focus,
            shows_focus: has_focus && state.focus_visible && state.window_focused,
            escaped: false,
        };
        for event in events {
            match event {
                WidgetEvent::Released {
                    button,
                    click_count,
                    is_over: true,
                    ..
                } => match button {
                    PointerButton::Primary => {
                        response.clicked |= sense.contains(Sense::CLICK);
                        response.double_clicked |= *click_count == 2;
                    }
                    PointerButton::Secondary => response.secondary_clicked = true,
                    PointerButton::Middle => response.middle_clicked = true,
                    PointerButton::Back | PointerButton::Forward => {}
                },
                WidgetEvent::Key(key) if key.modifiers == Modifiers::NONE => match key.key {
                    Key::Named(NamedKey::Enter) | Key::Character(' ') => {
                        response.clicked |= sense.contains(Sense::CLICK);
                    }
                    Key::Named(NamedKey::Escape) => response.escaped = true,
                    Key::Named(NamedKey::ContextMenu) => response.secondary_clicked = true,
                    _ => {}
                },
                // The other events are for the widgets that read them one by one, such as input boxes.
                _ => {}
            }
        }
        response
    }
}
