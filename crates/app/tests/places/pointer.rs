//! The pointer over a window drawn from the store: clicks, double clicks, drags and turns of the wheel at a point, the
//! intents they give, and the store changed by them.

#![allow(dead_code, reason = "each test file uses its own part of the pointer")]

use app::store::{Message, State};
use platform::accesskit::{Role, TreeUpdate};
use render::{Point, Rect};
use ui::{InputEvent, Modifiers, PointerButton, PointerButtonEvent, PointerMove, WheelDelta, WheelEvent};
use views::Intent;

use super::support::{Window, run};

/// The pointer moved to a point.
pub fn moved(position: Point) -> InputEvent {
    InputEvent::PointerMoved(PointerMove {
        position,
        modifiers: Modifiers::NONE,
    })
}

/// The main button at a point, as the click of a count.
fn button(position: Point, click_count: u32) -> PointerButtonEvent {
    PointerButtonEvent {
        button: PointerButton::Primary,
        position,
        modifiers: Modifiers::NONE,
        click_count,
    }
}

/// Clicks a point once, or twice for a double click, and gives the intents of all the frames.
pub fn click_at(window: &mut Window, state: &State, point: Point, clicks: u32) -> Vec<Intent> {
    let mut intents = window.draw(state, &[moved(point)]).0;
    for count in 1..=clicks {
        let press = button(point, count);
        intents.extend(
            window
                .draw(
                    state,
                    &[InputEvent::PointerPressed(press), InputEvent::PointerReleased(press)],
                )
                .0,
        );
    }
    intents
}

/// Presses the main button at a point, moves to another and lets go there, and gives the intents of all the frames.
pub fn drag(window: &mut Window, state: &State, from: Point, to: Point) -> Vec<Intent> {
    let mut intents = window.draw(state, &[moved(from)]).0;
    intents.extend(window.draw(state, &[InputEvent::PointerPressed(button(from, 1))]).0);
    let halfway = Point::new(f32::midpoint(from.x, to.x), f32::midpoint(from.y, to.y));
    intents.extend(window.draw(state, &[moved(halfway)]).0);
    intents.extend(window.draw(state, &[moved(to)]).0);
    intents.extend(window.draw(state, &[InputEvent::PointerReleased(button(to, 1))]).0);
    intents
}

/// Turns the wheel down by some lines over a point, and gives the intents of the frame.
pub fn wheel(window: &mut Window, state: &State, point: Point, lines: f32) -> Vec<Intent> {
    let mut intents = window.draw(state, &[moved(point)]).0;
    let turn = InputEvent::Wheel(WheelEvent {
        delta: WheelDelta::Lines { x: 0.0, y: lines },
        position: point,
        modifiers: Modifiers::NONE,
    });
    intents.extend(window.draw(state, &[turn]).0);
    window.now += std::time::Duration::from_secs(1);
    intents.extend(window.draw(state, &[]).0);
    intents
}

/// Hands the intents to the store, running the jobs they start.
pub fn apply(state: &mut State, intents: Vec<Intent>) {
    for intent in intents {
        let _effects = run(state, Message::Intent(intent));
    }
}

/// The rectangle of a node of the tree, which the test fails without.
pub fn bounds(tree: &TreeUpdate, role: Role, name: &str) -> Rect {
    let bounds = super::support::find(tree, role, name).bounds().unwrap();
    rect(bounds)
}

/// The rectangle of a node inside another that says some words, as a label does, which the test fails without.
pub fn said_at(tree: &TreeUpdate, words: &str, within: Rect) -> Rect {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.value() == Some(words) || node.label() == Some(words))
        .filter_map(|(_, node)| node.bounds().map(rect))
        .find(|found| found.x >= within.x && found.y >= within.y && found.bottom() <= within.bottom())
        .unwrap_or_else(|| panic!("nothing says {words}"))
}

/// Whether a node of the tree says some words.
pub fn says(tree: &TreeUpdate, words: &str) -> bool {
    tree.nodes
        .iter()
        .any(|(_, node)| node.value() == Some(words) || node.label() == Some(words))
}

/// The rows of the list drawn, from the top.
pub fn rows_drawn(tree: &TreeUpdate) -> Vec<Rect> {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::ListBoxOption)
        .filter_map(|(_, node)| node.bounds())
        .map(rect)
        .collect()
}

/// A rectangle of the tree in logical pixels.
#[allow(
    clippy::cast_possible_truncation,
    reason = "a window is far smaller than an f32 counts"
)]
fn rect(bounds: platform::accesskit::Rect) -> Rect {
    Rect::new(
        bounds.x0 as f32,
        bounds.y0 as f32,
        (bounds.x1 - bounds.x0) as f32,
        (bounds.y1 - bounds.y0) as f32,
    )
}

/// The middle of a rectangle.
pub fn middle(rect: Rect) -> Point {
    Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
}

/// The width of the graph kept for the workspace shown; none while the graph chooses it.
pub fn graph_width_of(state: &State) -> Option<f32> {
    let workspace = state.workspace.as_ref()?;
    app::persisted::timeline::panes_of(&state.persisted.panes, &workspace.folders)?.graph_width
}
