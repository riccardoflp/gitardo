//! Where each input of a frame goes, decided before the view runs: the pointer to the topmost widget under it as the
//! last frame laid them out, a drag to the widget it started on, the keys and the text to the widget with the focus,
//! Tab to the next widget that takes the focus.
//!
//! Deciding against the last frame is what lets the topmost widget win even when it is declared after the ones below
//! it; the application draws a frame after every batch of input, so the last frame is what the person saw.

use std::collections::HashMap;
use std::time::Duration;

use render::Point;

use super::hits::{Hits, Sense};
use crate::id::WidgetId;
use crate::input::{InputEvent, Key, KeyEvent, Modifiers, NamedKey, PointerButton, PointerButtonEvent, WheelDelta};

/// What happened to one widget, in the order it happened.
#[derive(Clone, Debug, PartialEq)]
pub enum WidgetEvent {
    /// A button was pressed over it.
    Pressed {
        /// The button.
        button: PointerButton,
        /// Where, in logical pixels of the window.
        position: Point,
        /// The modifier keys held.
        modifiers: Modifiers,
        /// 1 for a click, 2 for a double click, 3 for a triple click.
        click_count: u32,
    },
    /// A button pressed over it was released, over it or elsewhere.
    Released {
        /// The button.
        button: PointerButton,
        /// Where, in logical pixels of the window.
        position: Point,
        /// The count of clicks of its press.
        click_count: u32,
        /// Whether the pointer was still over it: a press and a release over the same widget make a click.
        is_over: bool,
    },
    /// The pointer moved while the main button pressed on it is held.
    Dragged {
        /// Where the pointer is, in logical pixels of the window.
        position: Point,
        /// The modifier keys held.
        modifiers: Modifiers,
    },
    /// The wheel turned over it, in logical pixels: positive `y` scrolls toward the bottom of the content.
    Wheel {
        /// How far, across and down.
        delta: Point,
    },
    /// A key was pressed while it has the focus.
    Key(KeyEvent),
    /// Text was typed while it has the focus.
    Text(String),
    /// Text was pasted while it has the focus.
    Paste(String),
}

/// A press of the main button that the widget it started on still holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Active {
    /// The widget.
    pub(crate) id: WidgetId,
}

/// What the toolkit remembers of the pointer and the keyboard between frames.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Interaction {
    /// Where the pointer is, when it is over the window.
    pub(crate) pointer: Option<Point>,
    /// The widget the main button was pressed on, while it is held.
    pub(crate) active: Option<Active>,
    /// The widget that covers where the main button was pressed, while it is held, even one that takes no press: a
    /// label of a row a drag starts from.
    pub(crate) pressed_on: Option<WidgetId>,
    /// The widget another button was pressed on, while it is held.
    pub(crate) other_press: Option<(WidgetId, PointerButton)>,
    /// The widget with the keyboard focus.
    pub(crate) focus: Option<WidgetId>,
    /// Whether the focus moved by the keyboard last, so that a ring shows where it is.
    pub(crate) focus_visible: bool,
    /// Whether the window has the keyboard.
    pub(crate) window_focused: bool,
    /// The widget under the pointer.
    pub(crate) hovered: Option<WidgetId>,
    /// The widget under the pointer and since when it is, for tooltips.
    pub(crate) hover_since: Option<(WidgetId, Duration)>,
    /// The widget a button was pressed on, whose tooltip stays hidden until the pointer leaves it.
    pub(crate) tooltip_hidden_on: Option<WidgetId>,
    /// The modifier keys held, as the last input said.
    pub(crate) modifiers: Modifiers,
    /// The widgets of the last frame.
    pub(crate) hits: Hits,
}

impl Default for Interaction {
    fn default() -> Interaction {
        Interaction {
            pointer: None,
            active: None,
            pressed_on: None,
            other_press: None,
            focus: None,
            focus_visible: false,
            window_focused: true,
            hovered: None,
            hover_since: None,
            tooltip_hidden_on: None,
            modifiers: Modifiers::NONE,
            hits: Hits::default(),
        }
    }
}

/// The input of a frame, sorted by the widget it goes to.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Routed {
    /// What each widget gets.
    pub(crate) events: HashMap<WidgetId, Vec<WidgetEvent>>,
    /// Every key pressed, for the shortcuts of the views.
    pub(crate) keys: Vec<KeyEvent>,
    /// Where each key of `keys` went and whether a widget used it, in the same order.
    pub(crate) key_routes: Vec<KeyRoute>,
    /// Where the pointer was pressed, to close what opened over the window when it is pressed elsewhere.
    pub(crate) presses: Vec<Point>,
    /// Whether the pointer moved in this frame, so that what it is over may take the place of what the keys chose.
    pub(crate) pointer_moved: bool,
}

impl Routed {
    /// Gives an event to a widget.
    fn send(&mut self, id: WidgetId, event: WidgetEvent) {
        self.events.entry(id).or_default().push(event);
    }
}

/// Where a key pressed in a frame went, and whether the widget it went to used it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct KeyRoute {
    /// The widget with the focus when it was pressed, if any.
    pub(crate) owner: Option<WidgetId>,
    /// Whether a widget used it, so that it is not a shortcut of the view; Tab, which moves the focus, is always used.
    pub(crate) is_claimed: bool,
}

impl Routed {
    /// The keys pressed in this frame that no widget used, in order: the shortcuts of the view.
    pub(crate) fn unclaimed(&self) -> Vec<KeyEvent> {
        self.keys
            .iter()
            .zip(&self.key_routes)
            .filter(|(_, route)| !route.is_claimed)
            .map(|(key, _)| *key)
            .collect()
    }

    /// Notes that a widget used a key it was sent: the first such press not yet used.
    pub(crate) fn claim(&mut self, id: WidgetId, key: KeyEvent) {
        let found = self
            .keys
            .iter()
            .zip(self.key_routes.iter_mut())
            .find(|(pressed, route)| **pressed == key && route.owner == Some(id) && !route.is_claimed);
        if let Some((_, route)) = found {
            route.is_claimed = true;
        }
    }
}

/// What a frame needs to know to sort its input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RouteSettings {
    /// The height of a line of text, which a turn of the wheel by one line scrolls.
    pub(crate) line_height: f32,
    /// The time of the frame.
    pub(crate) now: Duration,
}

/// Sorts the input of a frame by the widget it goes to, updating what is remembered of the pointer and the focus.
pub(crate) fn route(state: &mut Interaction, events: &[InputEvent], settings: RouteSettings) -> Routed {
    let mut routed = Routed::default();
    for event in events {
        match event {
            InputEvent::PointerMoved(moved) => {
                state.pointer = Some(moved.position);
                state.modifiers = moved.modifiers;
                routed.pointer_moved = true;
                if let Some(active) = state.active {
                    let dragged = WidgetEvent::Dragged {
                        position: moved.position,
                        modifiers: moved.modifiers,
                    };
                    routed.send(active.id, dragged);
                }
            }
            InputEvent::PointerPressed(press) => pressed(state, &mut routed, press),
            InputEvent::PointerReleased(release) => released(state, &mut routed, release),
            InputEvent::Wheel(wheel) => {
                state.pointer = Some(wheel.position);
                state.modifiers = wheel.modifiers;
                let delta = wheel_pixels(wheel.delta, wheel.modifiers, settings.line_height);
                if let Some(hit) = state.hits.topmost(wheel.position, Sense::SCROLL) {
                    routed.send(hit.id, WidgetEvent::Wheel { delta });
                }
            }
            InputEvent::PointerLeft => state.pointer = None,
            InputEvent::KeyPressed(key) => {
                state.modifiers = key.modifiers | modifier_of(key.key);
                key_pressed(state, &mut routed, *key);
            }
            InputEvent::KeyReleased(key) => state.modifiers = key.modifiers.without(modifier_of(key.key)),
            InputEvent::TextTyped(text) => to_focus(state, &mut routed, WidgetEvent::Text(text.clone())),
            InputEvent::Paste(text) => to_focus(state, &mut routed, WidgetEvent::Paste(text.clone())),
            InputEvent::FocusGained => state.window_focused = true,
            InputEvent::FocusLost => {
                state.window_focused = false;
                state.active = None;
                state.pressed_on = None;
                state.modifiers = Modifiers::NONE;
            }
        }
    }
    update_hover(state, settings.now);
    routed
}

/// The modifier a key is, when it is one: what holding it adds to the keys held.
fn modifier_of(key: Key) -> Modifiers {
    match key {
        Key::Named(NamedKey::Shift) => Modifiers::SHIFT,
        Key::Named(NamedKey::Control) => Modifiers::CONTROL,
        Key::Named(NamedKey::Alt) => Modifiers::ALT,
        Key::Named(NamedKey::Meta) => Modifiers::META,
        // Any other key is not a modifier, and adds none.
        _ => Modifiers::NONE,
    }
}

/// A press of a button: to the topmost widget under it, which holds the main button and takes the focus.
fn pressed(state: &mut Interaction, routed: &mut Routed, press: &PointerButtonEvent) {
    state.pointer = Some(press.position);
    state.modifiers = press.modifiers;
    routed.presses.push(press.position);
    state.tooltip_hidden_on = state.hits.covering(press.position).map(|hit| hit.id);
    let target = state
        .hits
        .topmost(press.position, Sense::CLICK | Sense::DRAG | Sense::FOCUS)
        .copied();
    if press.button == PointerButton::Primary {
        state.focus = target.filter(|hit| hit.sense.contains(Sense::FOCUS)).map(|hit| hit.id);
        state.focus_visible = false;
        state.active = target.map(|hit| Active { id: hit.id });
        state.pressed_on = state.hits.covering(press.position).map(|hit| hit.id);
    } else {
        state.other_press = target.map(|hit| (hit.id, press.button));
    }
    if let Some(hit) = target {
        let event = WidgetEvent::Pressed {
            button: press.button,
            position: press.position,
            modifiers: press.modifiers,
            click_count: press.click_count,
        };
        routed.send(hit.id, event);
    }
}

/// A release of a button: to the widget it was pressed on, saying whether it is still over it.
fn released(state: &mut Interaction, routed: &mut Routed, release: &PointerButtonEvent) {
    state.pointer = Some(release.position);
    let holder = if release.button == PointerButton::Primary {
        state.pressed_on = None;
        state.active.take().map(|active| active.id)
    } else {
        state
            .other_press
            .take()
            .filter(|(_, button)| *button == release.button)
            .map(|(id, _)| id)
    };
    let Some(id) = holder else {
        return;
    };
    let over = state
        .hits
        .topmost(release.position, Sense::CLICK | Sense::DRAG | Sense::FOCUS);
    let is_over = over.map(|hit| hit.id) == Some(id);
    let event = WidgetEvent::Released {
        button: release.button,
        position: release.position,
        click_count: release.click_count,
        is_over,
    };
    routed.send(id, event);
}

/// A key: Tab and Shift+Tab move the focus, but from a widget that takes every key; any other key goes to the widget
/// with the focus.
fn key_pressed(state: &mut Interaction, routed: &mut Routed, key: KeyEvent) {
    routed.keys.push(key);
    let is_tab = key.key == Key::Named(NamedKey::Tab);
    let is_kept = state.focus.is_some_and(|id| state.hits.takes_all_keys(id));
    if is_tab && !is_kept && (key.modifiers == Modifiers::NONE || key.modifiers == Modifiers::SHIFT) {
        let order = state.hits.focus_order();
        state.focus = next_focus(&order, state.focus, key.modifiers.shift());
        state.focus_visible = true;
        routed.key_routes.push(KeyRoute {
            owner: None,
            is_claimed: true,
        });
        return;
    }
    routed.key_routes.push(KeyRoute {
        owner: state.focus,
        is_claimed: false,
    });
    if let Some(id) = state.focus {
        routed.send(id, WidgetEvent::Key(key));
    }
}

/// Gives an event to the widget with the focus, when one has it.
fn to_focus(state: &Interaction, routed: &mut Routed, event: WidgetEvent) {
    if let Some(id) = state.focus {
        routed.send(id, event);
    }
}

/// The widget the focus goes to from another by Tab, or by Shift+Tab going back; round from the last to the first.
pub(crate) fn next_focus(order: &[WidgetId], from: Option<WidgetId>, is_backward: bool) -> Option<WidgetId> {
    let count = order.len();
    if count == 0 {
        return None;
    }
    let position = from.and_then(|id| order.iter().position(|candidate| *candidate == id));
    let next = match (position, is_backward) {
        (None, false) => 0,
        (None, true) => count - 1,
        (Some(at), false) => (at + 1) % count,
        (Some(at), true) => (at + count - 1) % count,
    };
    order.get(next).copied()
}

/// How far a turn of the wheel scrolls, in logical pixels; Shift turns a vertical turn into a horizontal one.
fn wheel_pixels(delta: WheelDelta, modifiers: Modifiers, line_height: f32) -> Point {
    let (x, y) = match delta {
        WheelDelta::Lines { x, y } => (x * line_height, y * line_height),
        WheelDelta::Pixels { x, y } => (x, y),
    };
    if modifiers.shift() && x == 0.0 {
        Point::new(y, 0.0)
    } else {
        Point::new(x, y)
    }
}

/// Finds the widget under the pointer: the one holding the main button while it is held and the pointer is over it,
/// otherwise the topmost one.
pub(crate) fn update_hover(state: &mut Interaction, now: Duration) {
    let covering = state
        .pointer
        .and_then(|point| state.hits.covering(point))
        .map(|hit| hit.id);
    state.hovered = match state.active {
        Some(active) => covering.filter(|id| *id == active.id),
        None => covering,
    };
    let since = state.hover_since.filter(|(id, _)| Some(*id) == state.hovered);
    state.hover_since = since.or_else(|| state.hovered.map(|id| (id, now)));
    if state.tooltip_hidden_on != state.hovered {
        state.tooltip_hidden_on = None;
    }
}
