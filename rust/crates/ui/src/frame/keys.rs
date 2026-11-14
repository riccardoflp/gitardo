//! The keys of a frame as the view sees them: every key pressed, the ones no widget used, which are the view's
//! shortcuts, and the presses of the pointer that close what opened over the window.

use render::Rect;

use super::Ui;
use crate::id::WidgetId;
use crate::input::{Key, KeyEvent, Modifiers, NamedKey};
use crate::interaction::Sense;

impl Ui<'_> {
    /// Every key pressed in this frame, in order, whatever widget had the focus and whether it used the key.
    #[must_use]
    pub fn key_presses(&self) -> &[KeyEvent] {
        &self.routed.keys
    }

    /// The keys pressed in this frame that no widget used, in order: the shortcuts of the view, such as Ctrl+F or F5,
    /// which work while an input box has the focus as long as the box did not use the key. Widgets declared after
    /// this call may still use a key, so a view reads its shortcuts after its widgets.
    #[must_use]
    pub fn unclaimed_keys(&self) -> Vec<KeyEvent> {
        self.routed.unclaimed()
    }

    /// The keys a widget was sent in this frame and did not use, in order: what a widget that holds another, such as
    /// the picker around its input box, reads after it.
    #[must_use]
    pub fn unclaimed_keys_of(&self, id: WidgetId) -> Vec<KeyEvent> {
        self.routed
            .keys
            .iter()
            .zip(&self.routed.key_routes)
            .filter(|(_, route)| route.owner == Some(id) && !route.is_claimed)
            .map(|(key, _)| *key)
            .collect()
    }

    /// Says that a widget used a key it was sent, so that the key is not a shortcut of the view.
    pub fn claim_key(&mut self, id: WidgetId, key: KeyEvent) {
        self.routed.claim(id, key);
    }

    /// Says that a key pressed in this frame was used, whatever widget it was sent to: for what the toolkit does
    /// itself with a key, such as Escape cancelling a drag.
    pub(crate) fn claim_any_key(&mut self, key: KeyEvent) {
        let found = self
            .routed
            .keys
            .iter()
            .zip(self.routed.key_routes.iter_mut())
            .find(|(pressed, route)| **pressed == key && !route.is_claimed);
        if let Some((_, route)) = found {
            route.is_claimed = true;
        }
    }

    /// Claims the keys a widget declared with [`Ui::interact`] reacts to: Enter and Space when it clicks, and the key
    /// of context menus.
    pub(super) fn claim_response_keys(&mut self, id: WidgetId, sense: Sense) {
        let used: Vec<KeyEvent> = self
            .routed
            .keys
            .iter()
            .zip(&self.routed.key_routes)
            .filter(|(key, route)| route.owner == Some(id) && is_response_key(**key, sense))
            .map(|(key, _)| *key)
            .collect();
        for key in used {
            self.routed.claim(id, key);
        }
    }

    /// Whether the pointer was pressed in this frame outside a rectangle: what closes a menu or a picker.
    #[must_use]
    pub fn pressed_outside(&self, rect: Rect) -> bool {
        self.routed.presses.iter().any(|point| !rect.contains(*point))
    }

    /// Whether the pointer was pressed in this frame outside all of some rectangles.
    pub(crate) fn pressed_outside_all(&self, rects: &[Rect]) -> bool {
        self.routed
            .presses
            .iter()
            .any(|point| rects.iter().all(|rect| !rect.contains(*point)))
    }

    /// Whether the pointer moved in this frame.
    pub(crate) fn pointer_moved(&self) -> bool {
        self.routed.pointer_moved
    }
}

/// Whether a key is one a widget declared with [`Ui::interact`] reacts to.
fn is_response_key(key: KeyEvent, sense: Sense) -> bool {
    if key.modifiers != Modifiers::NONE {
        return false;
    }
    match key.key {
        Key::Named(NamedKey::Enter) | Key::Character(' ') => sense.contains(Sense::CLICK),
        Key::Named(NamedKey::ContextMenu) => true,
        _ => false,
    }
}
