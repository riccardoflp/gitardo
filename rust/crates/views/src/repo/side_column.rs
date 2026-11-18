//! The side column (`repo.side-column`, D56): one island on the left of the window, from under the header to the
//! status bar, with **Repositories** above (`repo.tabs`) and **Branches** under it, which is the branch rail
//! (`branch.pane`); each folds by its heading, which has the buttons of its section on its right and, but for the
//! first, a line above it; the line between two open sections is dragged to share the height.

use std::time::Duration;

use ui::layout::{Area, Insets, inset};
use ui::{AccessRole, Axis, Description, Label, Rect, Sense, Split, Splitter, TextKind, Tone, Ui};

use super::side_sections::{DraggedLine, SECTION_LEAST, SectionAsk, SectionPlace, section_places};
use super::tabs::{TabsState, open_button, rows_height, tabs_view};
use crate::graph::{GraphIntent, HoverTarget, Pane, SideSection};
use crate::icons;
use crate::intent::Intent;
use crate::refs::ReferencesState;
use crate::text::repo as words;

/// The height **Repositories** starts at, its heading included, unless its rows need less.
pub const REPOSITORIES_START: f32 = 160.0;
/// How far an arrow moves a line between the sections.
const STEP: f32 = 16.0;

/// What the side column shows.
#[derive(Clone, Debug, PartialEq)]
pub struct SideColumnState<'a> {
    /// The repositories open.
    pub tabs: TabsState<'a>,
    /// The branch rail of the repository shown, once it is read; empty before.
    pub branches: Option<Box<ReferencesState<'a>>>,
    /// Which sections are folded, and the height of **Repositories**.
    pub sections: SideSections,
}

/// Which sections of the side column are folded, and the height **Repositories** was dragged to, as remembered for the
/// workspace.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct SideSections {
    /// The height of **Repositories**, its heading included, as dragged; none where it starts.
    pub repositories_height: Option<f32>,
    /// Whether **Repositories** is folded to its heading.
    pub is_repositories_folded: bool,
    /// Whether **Branches** is folded to its heading.
    pub is_branches_folded: bool,
}

/// Where the sections of the side column go, from the top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SectionRects {
    /// **Repositories**.
    pub repositories: SectionPlace,
    /// **Branches**.
    pub branches: SectionPlace,
}

/// Where the headings, the lines and the sections go in a column, from the heights of a heading, of the rows of
/// **Repositories** and of a line: **Repositories** as high as it was dragged, or where it starts,
/// [`REPOSITORIES_START`] or as tall as its rows when they need less, while **Branches** is open under it; a folded
/// section is its heading alone, and the heading of **Branches** has its line above it whatever is folded.
#[must_use]
pub fn section_rects(column: Rect, sections: SideSections, (heading, rows): (f32, f32), line: f32) -> SectionRects {
    let asks = [
        SectionAsk {
            is_folded: sections.is_repositories_folded,
            wanted: sections
                .repositories_height
                .unwrap_or_else(|| repositories_start(heading, rows)),
        },
        SectionAsk {
            is_folded: sections.is_branches_folded,
            // The last section takes what is left.
            wanted: SECTION_LEAST,
        },
    ];
    let [repositories, branches] = section_places(column, asks, (heading, line));
    SectionRects { repositories, branches }
}

/// The height **Repositories** starts at, its heading included: [`REPOSITORIES_START`], or as tall as its rows when
/// they need less.
fn repositories_start(heading: f32, rows: f32) -> f32 {
    REPOSITORIES_START.min(heading + rows)
}

/// The side column over the rectangle its caller clipped it to: the heading of **Repositories** with its chevron and
/// **+**, its rows, the line above the heading of **Branches**, which is dragged while both sections are open, that
/// heading and the branch rail. A click on a heading folds or unfolds its section.
///
/// Feature: repo.side-column
pub fn side_column_view(state: &SideColumnState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    side_column(state, ui).0
}

/// The side column, and the reference of the branch rail the pointer rests on with since when (`graph.hover-history`).
pub(crate) fn side_column(
    state: &SideColumnState<'_>,
    ui: &mut Ui<'_>,
) -> (Vec<Intent>, Option<(HoverTarget, Duration)>) {
    let column = ui.current_clip();
    let theme = *ui.theme();
    let heading = heading_height(ui);
    let line = theme.sizes.border;
    let rows = rows_height(ui, &state.tabs);
    let sections = state.sections;
    let mut intents = Vec::new();
    let mut hover = None;
    ui.region(
        "side column",
        (column, AccessRole::Navigation, words::SIDE_COLUMN),
        |ui| {
            let rects = section_rects(column, sections, (heading, rows), line);
            let start = repositories_start(heading, rows);
            let lines = [&rects.repositories, &rects.branches];
            intents.extend(lines.into_iter().filter_map(|place| {
                let dragged = SectionLine {
                    place,
                    pane: Pane::Repositories,
                    start,
                };
                section_line(ui, column, &dragged)
            }));
            let open = heading_row(
                ui,
                rects.repositories.heading,
                (SideSection::Repositories, sections, heading),
            );
            intents.extend(open);
            intents.extend(open_button(
                state.tabs.not_open,
                ui,
                Area::new(rects.repositories.heading).right(heading),
            ));
            if let Some(rows) = rects.repositories.body {
                intents.extend(ui.clip(rows, |ui| tabs_view(&state.tabs, ui)));
            }
            let buttons = state
                .branches
                .as_ref()
                .map_or(0.0, |_| crate::branch::buttons_width(ui, heading));
            intents.extend(heading_row(
                ui,
                rects.branches.heading,
                (SideSection::Branches, sections, buttons),
            ));
            if let Some(references) = &state.branches {
                let room = Area::new(rects.branches.heading).right(buttons + theme.sizes.spacing.small);
                let room = inset(room, Insets::symmetric(theme.sizes.spacing.small, 0.0));
                intents.extend(crate::branch::pane_buttons(references, ui, room));
            }
            if let (Some(rail), Some(references)) = (rects.branches.body, &state.branches) {
                let (rail_intents, rail_hover) = ui.clip(rail, |ui| crate::branch::pane_view(references, ui));
                intents.extend(rail_intents);
                hover = rail_hover;
            }
        },
    );
    (intents, hover)
}

/// The lines of a section, and what the one under it changes when it is dragged.
struct SectionLine<'a> {
    /// Where the section is.
    place: &'a SectionPlace,
    /// The pane whose height the line under the section sets.
    pane: Pane,
    /// The height the section starts at, which a double click on the line brings back.
    start: f32,
}

/// The lines of a section: the one above its heading, drawn whatever is folded, and the one under it while that one is
/// dragged, or moved with the arrows, to change the height of the section; a double click puts the section back where
/// it starts.
fn section_line(ui: &mut Ui<'_>, column: Rect, section: &SectionLine<'_>) -> Option<Intent> {
    if let Some(rule) = section.place.line {
        ui.separator(rule, Axis::Horizontal);
    }
    let DraggedLine {
        top,
        height,
        room,
        least_after,
    } = section.place.dragged?;
    let line = ui.theme().sizes.border;
    let splitter = Splitter {
        split: Split::Rows,
        default_size: section.start,
        least_first: SECTION_LEAST,
        least_second: least_after,
        size: Some(height),
        step: STEP,
    };
    // The splitter counts from the top of the section, over the height it shares with the open sections under it.
    let shared = Rect::new(column.x, top, column.width, room + line);
    let response = ui.splitter_across(("side sections line", section.pane), shared, &splitter, line);
    let sized = |size| Intent::Graph(GraphIntent::SetPane(section.pane, size));
    if response.is_reset {
        Some(sized(None))
    } else if response.is_changed {
        Some(sized(Some(response.size)))
    } else {
        None
    }
}

/// The height of the heading of a section: a row, with room for buttons of at least 24 logical pixels.
pub(crate) fn heading_height(ui: &Ui<'_>) -> f32 {
    let sizes = ui.theme().sizes;
    sizes.control_height - sizes.spacing.small
}

/// The heading of a section: its chevron, down while it is open, and its name in capitals, short of the room its
/// buttons take on its right; a click folds or unfolds it.
fn heading_row(
    ui: &mut Ui<'_>,
    rect: Rect,
    (section, sections, buttons): (SideSection, SideSections, f32),
) -> Option<Intent> {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let (name, is_folded) = match section {
        SideSection::Repositories => (words::REPOSITORIES, sections.is_repositories_folded),
        SideSection::Branches => (words::BRANCHES, sections.is_branches_folded),
    };
    let id = ui.id(("side heading", section));
    let response = ui.interact(id, rect, Sense::CLICK | Sense::FOCUS);
    if response.contains_pointer {
        ui.fill_rect(rect, theme.colors.list_hover);
    }
    if response.shows_focus {
        ui.focus_ring(rect, theme.sizes.radius.medium);
    }
    let mut parts = Area::with_gap(inset(rect, Insets::symmetric(spacing.medium, 0.0)), spacing.small);
    let _ = parts.right(buttons);
    let chevron = if is_folded { icons::FOLDED } else { icons::UNFOLDED };
    ui.icon_in(parts.left(theme.sizes.icon), chevron, Tone::Muted);
    let _ = ui.label(
        parts.rest(),
        &Label::new(name).kind(TextKind::SmallStrong).tone(Tone::Muted),
    );
    ui.describe(
        id,
        rect,
        &Description::new(AccessRole::Button, name).expanded(!is_folded),
    );
    response
        .clicked
        .then_some(Intent::Graph(GraphIntent::FoldSection(section)))
}
