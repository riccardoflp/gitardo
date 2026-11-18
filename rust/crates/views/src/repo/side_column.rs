//! The side column (`repo.side-column`, D56, D71): one island on the left of the window, from under the header to the
//! status bar, with **Repositories** above (`repo.tabs`), **Changes** under it (`changes.section`) and **Branches**
//! last, which is the branch rail (`branch.pane`); each folds by its heading, which has the buttons of its section on
//! its right and, but for the first, a line above it; the line between two open sections is dragged to share the
//! height.

use std::time::Duration;

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Axis, Description, Label, Rect, Sense, Split, Splitter, TextKind, Tone, Ui};

use super::side_sections::{DraggedLine, SECTION_LEAST, SectionAsk, SectionPlace, section_places};
use super::tabs::{TabsState, open_button, rows_height, tabs_view};
use crate::changes::{ChangesState, section_count, section_height, section_view};
use crate::graph::{GraphIntent, HoverTarget, Pane, SideSection};
use crate::icons;
use crate::intent::Intent;
use crate::refs::ReferencesState;
use crate::text::repo as words;

/// The height **Repositories** starts at, its heading included, unless its rows need less.
pub const REPOSITORIES_START: f32 = 160.0;
/// The height **Changes** starts at, its heading included, unless what it shows needs less.
pub const CHANGES_START: f32 = 280.0;
/// How far an arrow moves a line between the sections.
const STEP: f32 = 16.0;

/// What the side column shows.
#[derive(Clone, Debug, PartialEq)]
pub struct SideColumnState<'a> {
    /// The repositories open.
    pub tabs: TabsState<'a>,
    /// The changes of the repository shown, as the Changes page has them; no repository before one is read.
    pub changes: Box<ChangesState<'a>>,
    /// The branch rail of the repository shown, once it is read; empty before.
    pub branches: Option<Box<ReferencesState<'a>>>,
    /// Which sections are folded, and the heights of **Repositories** and of **Changes**.
    pub sections: SideSections,
}

/// Which sections of the side column are folded, and the heights **Repositories** and **Changes** were dragged to, as
/// remembered for the workspace.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct SideSections {
    /// The height of **Repositories**, its heading included, as dragged; none where it starts.
    pub repositories_height: Option<f32>,
    /// The height of **Changes**, its heading included, as dragged; none where it starts.
    pub changes_height: Option<f32>,
    /// Whether **Repositories** is folded to its heading.
    pub is_repositories_folded: bool,
    /// Whether **Changes** is folded to its heading.
    pub is_changes_folded: bool,
    /// Whether **Branches** is folded to its heading.
    pub is_branches_folded: bool,
}

impl SideSections {
    /// Whether a section is folded to its heading.
    #[must_use]
    pub fn is_folded(&self, section: SideSection) -> bool {
        match section {
            SideSection::Repositories => self.is_repositories_folded,
            SideSection::Changes => self.is_changes_folded,
            SideSection::Branches => self.is_branches_folded,
        }
    }
}

/// Where the sections of the side column go, from the top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SectionRects {
    /// **Repositories**.
    pub repositories: SectionPlace,
    /// **Changes**.
    pub changes: SectionPlace,
    /// **Branches**.
    pub branches: SectionPlace,
}

/// The heights the places of the sections are worked out from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SectionHeights {
    /// The height of a heading.
    pub heading: f32,
    /// The height of a line above a heading.
    pub line: f32,
    /// The height of the rows of **Repositories**.
    pub repositories: f32,
    /// The height of what **Changes** shows under its heading; none while no repository is shown, when the section is
    /// its heading alone.
    pub changes: Option<f32>,
}

/// Where the headings, the lines and the sections go in a column: **Repositories** and **Changes** as high as they
/// were dragged, or where they start, [`REPOSITORIES_START`] and [`CHANGES_START`] or as tall as what they show when
/// it needs less, while a section is open under them; the last open section takes the rest; a folded section is its
/// heading alone, as **Changes** is while no repository is shown, and every heading but the first has its line above it
/// whatever is folded.
#[must_use]
pub fn section_rects(column: Rect, sections: SideSections, heights: &SectionHeights) -> SectionRects {
    let asks = [
        SectionAsk {
            is_folded: sections.is_repositories_folded,
            wanted: sections
                .repositories_height
                .unwrap_or_else(|| repositories_start(heights)),
        },
        SectionAsk {
            is_folded: sections.is_changes_folded || heights.changes.is_none(),
            wanted: sections.changes_height.unwrap_or_else(|| changes_start(heights)),
        },
        SectionAsk {
            is_folded: sections.is_branches_folded,
            // The last section takes what is left.
            wanted: SECTION_LEAST,
        },
    ];
    let [repositories, changes, branches] = section_places(column, asks, (heights.heading, heights.line));
    SectionRects {
        repositories,
        changes,
        branches,
    }
}

/// The height **Repositories** starts at, its heading included: [`REPOSITORIES_START`], or as tall as its rows when
/// they need less.
fn repositories_start(heights: &SectionHeights) -> f32 {
    REPOSITORIES_START.min(heights.heading + heights.repositories)
}

/// The height **Changes** starts at, its heading included: [`CHANGES_START`], or as tall as what it shows when that
/// needs less (Q768).
fn changes_start(heights: &SectionHeights) -> f32 {
    CHANGES_START.min(heights.heading + heights.changes.unwrap_or_default())
}

/// The side column over the rectangle its caller clipped it to: the heading of **Repositories** with its chevron and
/// **+**, its rows; the heading of **Changes** with the count of the changed files, and the changes; the heading of
/// **Branches** and the branch rail. Each heading but the first has a line above it, which is dragged while the
/// section above it and one under it are open. A click on a heading folds or unfolds its section.
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
    let heights = SectionHeights {
        heading: heading_height(ui),
        line: ui.theme().sizes.border,
        repositories: rows_height(ui, &state.tabs),
        changes: section_height(&state.changes, ui),
    };
    let mut intents = Vec::new();
    let mut hover = None;
    ui.region(
        "side column",
        (column, AccessRole::Navigation, words::SIDE_COLUMN),
        |ui| {
            let rects = section_rects(column, state.sections, &heights);
            let lines = [
                (&rects.repositories, Pane::Repositories, repositories_start(&heights)),
                (&rects.changes, Pane::ChangesSection, changes_start(&heights)),
                // The last section has no line under it to drag.
                (&rects.branches, Pane::ChangesSection, SECTION_LEAST),
            ];
            intents.extend(
                lines
                    .into_iter()
                    .filter_map(|(place, pane, start)| section_line(ui, column, &SectionLine { place, pane, start })),
            );
            intents.extend(repositories_section(state, ui, &rects.repositories));
            intents.extend(changes_section(state, ui, &rects.changes));
            let (rail_intents, rail_hover) = branches_section(state, ui, &rects.branches);
            intents.extend(rail_intents);
            hover = rail_hover;
        },
    );
    (intents, hover)
}

/// **Repositories**: its heading with **+**, and its rows while it is open.
fn repositories_section(state: &SideColumnState<'_>, ui: &mut Ui<'_>, place: &SectionPlace) -> Vec<Intent> {
    let mut intents = Vec::new();
    let heading = place.heading.height;
    let title = HeadingState {
        section: SideSection::Repositories,
        is_folded: state.sections.is_repositories_folded,
        count: None,
        buttons: heading,
    };
    intents.extend(heading_row(ui, place.heading, &title));
    intents.extend(open_button(
        state.tabs.not_open,
        ui,
        Area::new(place.heading).right(heading),
    ));
    if let Some(rows) = place.body {
        intents.extend(ui.clip(rows, |ui| tabs_view(&state.tabs, ui)));
    }
    intents
}

/// **Changes**: its heading with the count of the changed files, and the changes while it is open (`changes.section`).
fn changes_section(state: &SideColumnState<'_>, ui: &mut Ui<'_>, place: &SectionPlace) -> Vec<Intent> {
    let mut intents = Vec::new();
    let title = HeadingState {
        section: SideSection::Changes,
        is_folded: state.sections.is_changes_folded,
        count: section_count(&state.changes),
        buttons: 0.0,
    };
    intents.extend(heading_row(ui, place.heading, &title));
    if let Some(body) = place.body {
        intents.extend(ui.clip(body, |ui| section_view(&state.changes, ui)));
    }
    intents
}

/// **Branches**: its heading with the buttons of the branch rail, and the rail while it is open; gives the reference
/// of the rail the pointer rests on too.
fn branches_section(
    state: &SideColumnState<'_>,
    ui: &mut Ui<'_>,
    place: &SectionPlace,
) -> (Vec<Intent>, Option<(HoverTarget, Duration)>) {
    let mut intents = Vec::new();
    let spacing = ui.theme().sizes.spacing;
    let buttons = state
        .branches
        .as_ref()
        .map_or(0.0, |_| crate::branch::buttons_width(ui, place.heading.height));
    let title = HeadingState {
        section: SideSection::Branches,
        is_folded: state.sections.is_branches_folded,
        count: None,
        buttons,
    };
    intents.extend(heading_row(ui, place.heading, &title));
    let Some(references) = &state.branches else {
        return (intents, None);
    };
    let room = Area::new(place.heading).right(buttons + spacing.small);
    let room = inset(room, Insets::symmetric(spacing.small, 0.0));
    intents.extend(crate::branch::pane_buttons(references, ui, room));
    let Some(rail) = place.body else {
        return (intents, None);
    };
    let (rail_intents, hover) = ui.clip(rail, |ui| crate::branch::pane_view(references, ui));
    intents.extend(rail_intents);
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

/// What the heading of a section is drawn from.
struct HeadingState {
    /// The section.
    section: SideSection,
    /// Whether it is folded.
    is_folded: bool,
    /// The count beside its name, for **Changes** once a repository is shown.
    count: Option<usize>,
    /// The room its buttons take on its right.
    buttons: f32,
}

/// The heading of a section: its chevron, down while it is open, its name in capitals and its count, short of the
/// room its buttons take on its right; a click folds or unfolds it.
fn heading_row(ui: &mut Ui<'_>, rect: Rect, heading: &HeadingState) -> Option<Intent> {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let name = match heading.section {
        SideSection::Repositories => words::REPOSITORIES,
        SideSection::Changes => words::CHANGES,
        SideSection::Branches => words::BRANCHES,
    };
    let id = ui.id(("side heading", heading.section));
    let response = ui.interact(id, rect, Sense::CLICK | Sense::FOCUS);
    if response.contains_pointer {
        ui.fill_rect(rect, theme.colors.list_hover);
    }
    if response.shows_focus {
        ui.focus_ring(rect, theme.sizes.radius.medium);
    }
    let mut parts = Area::with_gap(inset(rect, Insets::symmetric(spacing.medium, 0.0)), spacing.small);
    let _ = parts.right(heading.buttons);
    let chevron = if heading.is_folded {
        icons::FOLDED
    } else {
        icons::UNFOLDED
    };
    ui.icon_in(parts.left(theme.sizes.icon), chevron, Tone::Muted);
    // A section with nothing to count says its name alone.
    let count = heading.count.filter(|count| *count > 0).map(|count| count.to_string());
    let style = ui.text_style(TextKind::SmallStrong);
    let titled = parts.left(ui.measure(name, style).min(parts.rest().width));
    let _ = ui.label(titled, &Label::new(name).kind(TextKind::SmallStrong).tone(Tone::Muted));
    let described = Description::new(AccessRole::Button, name).expanded(!heading.is_folded);
    if let Some(count) = &count {
        let size = (ui.measure(count, style), ui.line_height(style));
        let cell = place(parts.rest(), size, (Align::Start, Align::Center));
        let _ = ui.label(cell, &Label::new(count).kind(TextKind::SmallStrong).tone(Tone::Muted));
        ui.describe(id, rect, &described.value(count));
    } else {
        ui.describe(id, rect, &described);
    }
    response
        .clicked
        .then_some(Intent::Graph(GraphIntent::FoldSection(heading.section)))
}
