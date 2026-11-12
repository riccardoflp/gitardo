//! What the screen reader asks of a widget, done as the pointer or the keyboard would: a click is a press and a
//! release in the middle of the widget, given to it in the next frame, and the focus moves as Tab would move it.

use accesskit::{Action, ActionRequest};
use render::Point;

use crate::input::{Modifiers, PointerButton};
use crate::interaction::WidgetEvent;
use crate::memory::UiMemory;

impl UiMemory {
    /// Takes a request of the screen reader about a widget of the last frame: a click, which the widget gets in the
    /// next frame as a click of the pointer in its middle, or the focus, which moves to it at once. Says whether the
    /// request was taken, when the application should draw a frame.
    pub fn accessibility_action(&mut self, request: &ActionRequest) -> bool {
        let Some(widget) = self
            .access
            .nodes
            .iter()
            .find(|widget| widget.id.number() == request.target_node.0)
        else {
            return false;
        };
        if widget.is_disabled {
            return false;
        }
        let (owner, center, role) = (widget.owner, widget.rect.center(), widget.role);
        let is_focusable = self.interaction.hits.is_focusable(owner);
        match request.action {
            Action::Click if role.is_clickable() => {
                if is_focusable {
                    self.interaction.focus = Some(owner);
                }
                self.access.pending.extend(click(center).map(|event| (owner, event)));
                true
            }
            Action::Focus if is_focusable => {
                self.interaction.focus = Some(owner);
                self.interaction.focus_visible = true;
                true
            }
            _ => false,
        }
    }
}

/// A click of the main button at a point: its press and its release over the same widget.
fn click(position: Point) -> [WidgetEvent; 2] {
    [
        WidgetEvent::Pressed {
            button: PointerButton::Primary,
            position,
            modifiers: Modifiers::NONE,
            click_count: 1,
        },
        WidgetEvent::Released {
            button: PointerButton::Primary,
            position,
            click_count: 1,
            is_over: true,
        },
    ]
}
