//! Branch names that stay at the top of the list (`graph.sticky-branch-names`): scrolled past the tip of a branch whose
//! line goes on down, its name stays at the top, in the color of its line.

use std::time::Duration;

use model::CommitId;
use model::graph::{Half, Row};
use ui::{Badge, BadgeFill, Icon, Rect, Sense, Tone, Ui};

use super::intent::{GraphIntent, HoverTarget, HoverWay};
use super::layout::TimelineLayout;
use crate::intent::Intent;
use crate::text::timeline as words;

/// A name at the top of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StickyName<'a> {
    /// The commit at the tip.
    pub commit: &'a CommitId,
    /// Its first label.
    pub name: &'a str,
    /// The color of the line.
    pub color: usize,
}

/// The names at the top of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StickyNames<'a> {
    /// The names, from the left.
    pub names: Vec<StickyName<'a>>,
}

/// The names of the tips of the lines that come down into the first row in sight from above it, each tip once, in the
/// order of their lanes; none at the very top of the list.
#[must_use]
pub fn sticky_names<'a>(layout: &'a TimelineLayout, rows: &'a [Row], first: usize) -> StickyNames<'a> {
    let mut names: Vec<StickyName<'_>> = Vec::new();
    if first == 0 {
        return StickyNames { names };
    }
    let Some(tips) = layout.tips.get(first) else {
        return StickyNames { names };
    };
    for (lane, tip) in tips.iter().enumerate() {
        let Some(tip) = tip.filter(|tip| *tip < first) else {
            continue;
        };
        let Some(commit) = layout.id_at(rows, tip) else {
            continue;
        };
        if names.iter().any(|name| name.commit == commit) {
            continue;
        }
        let Some(label) = layout.labels.get(commit).and_then(|labels| labels.first()) else {
            continue;
        };
        let color = layout
            .cells
            .get(first)
            .and_then(|cell| {
                cell.lines
                    .iter()
                    .find(|line| line.half == Half::Upper && line.from == lane)
            })
            .map(|line| line.color)
            .or_else(|| layout.cells.get(tip).map(|cell| cell.color))
            .unwrap_or(0);
        names.push(StickyName {
            commit,
            name: &label.name,
            color,
        });
    }
    StickyNames { names }
}

/// What the sticky names report: a click, and the name under the pointer for its history.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StickyReport {
    /// The intents.
    pub intents: Vec<Intent>,
    /// The name under the pointer, with since when it rests there.
    pub label: Option<(HoverTarget, Duration)>,
}

/// The names at the top of the rectangle its caller clipped them to, beside the graph: a click scrolls up to the tip,
/// centered, and selects it; resting on one brings out its history.
///
/// Feature: graph.sticky-branch-names
pub fn sticky_branch_names_view(state: &StickyNames<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    sticky_names_drawn(state, ui).intents
}

/// The names drawn, with what they report.
pub(crate) fn sticky_names_drawn(state: &StickyNames<'_>, ui: &mut Ui<'_>) -> StickyReport {
    let mut report = StickyReport::default();
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let mut x = rect.x;
    let top = rect.y + theme.sizes.spacing.small;
    for name in &state.names {
        let badge = Badge {
            text: name.name,
            icons: &[Icon::ArrowUp],
            tone: Tone::Lane(name.color),
            fill: BadgeFill::Outline,
        };
        let width = ui.badge_width(&badge);
        let height = ui.badge_height();
        let cell = Rect::new(x, top, width, height);
        ui.fill_rounded_rect(cell, height * 0.5, theme.colors.page_background);
        let drawn = ui.badge(cell, &badge);
        let id = ui.id(("sticky", name.commit.as_str()));
        let response = ui.interact(id, drawn, Sense::CLICK | Sense::HOVER);
        ui.tooltip(&response, &words::go_up_to(name.name));
        if response.clicked {
            report
                .intents
                .push(Intent::Graph(GraphIntent::GoTo(name.commit.clone())));
        }
        if let Some(since) = ui.hovered_since(&response) {
            let target = HoverTarget {
                commit: name.commit.clone(),
                way: HoverWay::Reaches,
            };
            report.label = Some((target, since));
        }
        x += width + theme.sizes.spacing.medium;
    }
    report
}
