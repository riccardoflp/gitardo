//! The side column in the window (`repo.side-column`): beside what is shown, at its width with the gap after it that
//! resizes it, or over the timeline in the narrow layout, and put away while a Diff page beside the inspector is shown.

use ui::{Rect, Ui};

use super::{Content, SidePlace, TimelineState, WindowState};
use crate::graph::{GraphIntent, HoverTarget, PaneLayout, PaneSizes, ReferencesShown, pane_rects, resize_panes_view};
use crate::intent::Intent;
use crate::islands::island;
use crate::line::is_narrow;
use crate::repo::{SideColumnState, side_column};

/// Whether what is shown puts the side column away while it is: a Diff page beside the inspector
/// (`diff.beside-inspector`), or beside a comparison (`compare.beside-diff`).
pub(super) fn puts_side_away(content: &Content<'_>) -> bool {
    match content {
        Content::Timeline(timeline) => matches!(timeline, TimelineState::Repository { beside: Some(_), .. }),
        Content::Page(host) => host.page.puts_side_away(),
    }
}

/// The side column on the left of what is under the header, at its width, with the gap after it that is dragged to
/// resize it; gives what is left on its right and the reference of the branch rail the pointer rests on.
pub(super) fn side_beside(
    side: &SideColumnState<'_>,
    place: SidePlace,
    ui: &mut Ui<'_>,
    (body, gap): (Rect, f32),
    intents: &mut Vec<Intent>,
) -> (Rect, Option<(HoverTarget, std::time::Duration)>) {
    let layout = PaneLayout {
        body,
        sizes: PaneSizes {
            rail: place.width,
            ..PaneSizes::default()
        },
        icon_rail: None,
        is_rail_hidden: false,
        is_inspector_over: false,
        has_inspector: false,
        gap,
        bottom: 0.0,
    };
    let rects = pane_rects(&layout, gap);
    let Some(rect) = rects.rail else {
        return (body, None);
    };
    let (side_intents, hover) = island(ui, rect, |ui| side_column(side, ui));
    intents.extend(side_intents);
    intents.extend(resize_panes_view(&layout, ui));
    (rects.main, hover)
}

/// The side column opened over the timeline in the narrow layout, as wide as it was, until a click outside it or
/// `Escape` (`repo.side-column`).
pub(super) fn side_over(
    side: &SideColumnState<'_>,
    place: SidePlace,
    ui: &mut Ui<'_>,
    (body, gap): (Rect, f32),
    intents: &mut Vec<Intent>,
) {
    let least = crate::graph::RAIL_LEAST;
    let width = place
        .width
        .unwrap_or(crate::graph::RAIL_START)
        .clamp(least, (body.width - gap).max(least));
    let rect = Rect::new(body.x, body.y, width.min(body.width), body.height);
    ui.layer(ui::Layer::Popup, |ui| {
        let (side_intents, _) = island(ui, rect, |ui| side_column(side, ui));
        intents.extend(side_intents);
        if ui.pressed_outside(rect) {
            intents.push(Intent::Graph(GraphIntent::CloseSideColumn));
        }
    });
}

/// Whether the button of the side column at the top of the timeline is pressed: the side column is shown beside the
/// timeline, or over it in the narrow layout; none without a workspace.
pub(super) fn side_toggle(window: &WindowState<'_>, ui: &Ui<'_>) -> Option<ReferencesShown> {
    let is_narrow = is_narrow(ui);
    window
        .side
        .as_ref()
        .map(|_| ReferencesShown::of(window.side_place, is_narrow))
}
