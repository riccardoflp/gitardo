//! The branch rail, the commits and the inspector side by side, or the inspector over the commits in the narrow layout,
//! and the lines between them that are dragged to resize them (`graph.resize-panes`, `graph.toggle-references`).

use ui::{Rect, Split, Splitter, Ui};

use super::intent::{GraphIntent, Pane};
use crate::intent::Intent;

/// Where the branch rail starts, and where a double click puts it back.
pub const RAIL_START: f32 = 210.0;
/// The least width of the branch rail.
pub const RAIL_LEAST: f32 = 120.0;
/// The most of the window the branch rail takes.
pub const RAIL_MOST: f32 = 0.35;
/// Where the inspector beside the commits starts.
pub const INSPECTOR_START: f32 = 320.0;
/// The least width of the inspector beside the commits.
pub const INSPECTOR_LEAST: f32 = 200.0;
/// The most of the window it takes.
pub const INSPECTOR_MOST: f32 = 0.55;
/// The least height of the inspector over the commits.
pub const OVER_LEAST: f32 = 80.0;
/// The most of the height it takes.
pub const OVER_MOST: f32 = 0.8;
/// How far an arrow moves a line.
pub const STEP: f32 = 16.0;

/// The sizes of the panes, as dragged; none where a pane is where it starts.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct PaneSizes {
    /// The width of the branch rail.
    pub rail: Option<f32>,
    /// The width of the inspector beside the commits.
    pub inspector: Option<f32>,
    /// The height of the inspector over the commits.
    pub inspector_over: Option<f32>,
}

/// How the panes are laid out in a frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaneLayout {
    /// The rectangle under the header the panes share.
    pub body: Rect,
    /// The sizes dragged.
    pub sizes: PaneSizes,
    /// The width of the branch rail when it shows only its icons; none when it can be resized.
    pub icon_rail: Option<f32>,
    /// Whether the branch rail is put away.
    pub is_rail_hidden: bool,
    /// Whether the inspector is over the commits, in the narrow layout, rather than beside them.
    pub is_inspector_over: bool,
    /// Whether there is an inspector: over the commits, only while something is selected.
    pub has_inspector: bool,
}

/// Where each pane goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaneRects {
    /// The branch rail; none while it is put away.
    pub rail: Option<Rect>,
    /// The commits.
    pub main: Rect,
    /// The inspector, when there is one.
    pub inspector: Option<Rect>,
}

/// A size kept within its least and its most, and the room there is: what a splitter does with it.
fn clamp(size: f32, (least, most): (f32, f32)) -> f32 {
    size.clamp(least, most.max(least))
}

/// The splitter of the line after the branch rail, over the body.
fn rail_splitter(layout: &PaneLayout, line: f32) -> Splitter {
    let width = layout.body.width;
    let most = width * RAIL_MOST;
    Splitter {
        split: Split::Columns,
        default_size: RAIL_START,
        least_first: RAIL_LEAST,
        least_second: (width - line - most).max(0.0),
        size: Some(clamp(layout.sizes.rail.unwrap_or(RAIL_START), (RAIL_LEAST, most))),
        step: STEP,
    }
}

/// The splitter of the line before the inspector beside the commits, over what is right of the rail.
fn inspector_splitter(layout: &PaneLayout, rest: Rect, line: f32) -> Splitter {
    let most = layout.body.width * INSPECTOR_MOST;
    let inspector = clamp(
        layout.sizes.inspector.unwrap_or(INSPECTOR_START),
        (INSPECTOR_LEAST, most),
    );
    Splitter {
        split: Split::Columns,
        default_size: (rest.width - line - INSPECTOR_START).max(0.0),
        least_first: (rest.width - line - most).max(0.0),
        least_second: INSPECTOR_LEAST,
        size: Some((rest.width - line - inspector).max(0.0)),
        step: STEP,
    }
}

/// The splitter of the line above the inspector over the commits.
fn over_splitter(layout: &PaneLayout, main: Rect, line: f32) -> Splitter {
    let height = main.height;
    let most = height * OVER_MOST;
    let start = height * 0.5;
    let inspector = clamp(layout.sizes.inspector_over.unwrap_or(start), (OVER_LEAST, most));
    Splitter {
        split: Split::Rows,
        default_size: (height - line - start).max(0.0),
        least_first: (height - line - most).max(0.0),
        least_second: OVER_LEAST,
        size: Some((height - line - inspector).max(0.0)),
        step: STEP,
    }
}

/// Where each pane goes: the rail at its width, or its icons, or none; the inspector beside the commits at its width,
/// or over them at its height; the commits in the rest.
#[must_use]
pub fn pane_rects(layout: &PaneLayout, line: f32) -> PaneRects {
    let body = layout.body;
    let (rail, rest) = match (layout.is_rail_hidden, layout.icon_rail) {
        (true, _) => (None, body),
        (false, Some(width)) => split_columns(body, width, 0.0),
        (false, None) => split_columns(body, rail_splitter(layout, line).size.unwrap_or(RAIL_START), line),
    };
    if !layout.has_inspector {
        return PaneRects {
            rail,
            main: rest,
            inspector: None,
        };
    }
    let (main, inspector) = if layout.is_inspector_over {
        let splitter = over_splitter(layout, rest, line);
        let top = splitter.size.unwrap_or(0.0);
        let inspector = Rect::new(
            rest.x,
            rest.y + top + line,
            rest.width,
            (rest.height - top - line).max(0.0),
        );
        (rest, inspector)
    } else {
        let size = inspector_splitter(layout, rest, line).size.unwrap_or(0.0);
        let (main, inspector) = split_columns(rest, size, line);
        (main.unwrap_or(rest), inspector)
    };
    PaneRects {
        rail,
        main,
        inspector: Some(inspector),
    }
}

/// A rectangle cut in two columns after a width, with a line between them.
fn split_columns(rect: Rect, width: f32, line: f32) -> (Option<Rect>, Rect) {
    let width = width.min(rect.width);
    (
        Some(Rect::new(rect.x, rect.y, width, rect.height)),
        Rect::new(
            rect.x + width + line,
            rect.y,
            (rect.width - width - line).max(0.0),
            rect.height,
        ),
    )
}

/// The lines between the panes, over the body: dragged, or moved with the arrows, they say the new size; a double
/// click puts a pane back where it starts.
///
/// Feature: graph.resize-panes
pub fn resize_panes_view(state: &PaneLayout, ui: &mut Ui<'_>) -> Vec<Intent> {
    let line = ui.theme().sizes.border;
    let mut intents = Vec::new();
    let body = state.body;
    let rest = if state.is_rail_hidden || state.icon_rail.is_some() {
        pane_rects(state, line).rail.map_or(body, |rail| {
            Rect::new(rail.right(), body.y, body.right() - rail.right(), body.height)
        })
    } else {
        let response = ui.splitter("rail line", body, &rail_splitter(state, line));
        push_change(
            &mut intents,
            Pane::Rail,
            (response.is_reset, response.is_changed),
            response.size,
        );
        response.second
    };
    if !state.has_inspector {
        return intents;
    }
    if state.is_inspector_over {
        let response = ui.splitter("inspector over line", rest, &over_splitter(state, rest, line));
        let size = (rest.height - line - response.size).max(0.0);
        push_change(
            &mut intents,
            Pane::InspectorOver,
            (response.is_reset, response.is_changed),
            size,
        );
    } else {
        let response = ui.splitter("inspector line", rest, &inspector_splitter(state, rest, line));
        let size = (rest.width - line - response.size).max(0.0);
        push_change(
            &mut intents,
            Pane::Inspector,
            (response.is_reset, response.is_changed),
            size,
        );
    }
    intents
}

/// The intent of a line moved, or put back.
fn push_change(intents: &mut Vec<Intent>, pane: Pane, (is_reset, is_changed): (bool, bool), size: f32) {
    if is_reset {
        intents.push(Intent::Graph(GraphIntent::SetPane(pane, None)));
    } else if is_changed {
        intents.push(Intent::Graph(GraphIntent::SetPane(pane, Some(size))));
    }
}
