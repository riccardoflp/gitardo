//! The side column (`repo.side-column`, D56): one island on the left of the window, from under the header to the
//! status bar, with **Repositories** above (`repo.tabs`) and **Branches** under it, which is the branch rail
//! (`branch.pane`); each folds by its heading, and the line between them is dragged to share the height.

use std::time::Duration;

use ui::layout::{Area, Insets, inset};
use ui::{AccessRole, Axis, Description, Label, Rect, Sense, Split, Splitter, TextKind, Tone, Ui};

use super::tabs::{TabsState, open_button, rows_height, tabs_view};
use crate::graph::{GraphIntent, HoverTarget, Pane, SideSection};
use crate::icons;
use crate::intent::Intent;
use crate::refs::ReferencesState;
use crate::text::repo as words;

/// The height **Repositories** starts at, its heading included, unless its rows need less.
pub const REPOSITORIES_START: f32 = 160.0;
/// The least height of either section while both are open.
pub const SECTION_LEAST: f32 = 64.0;
/// How far an arrow moves the line between the sections.
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

/// Where the parts of the side column go.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SectionRects {
    /// The heading of **Repositories**.
    pub repositories_heading: Rect,
    /// Its rows, while it is open.
    pub repositories: Option<Rect>,
    /// The heading of **Branches**.
    pub branches_heading: Rect,
    /// The branch rail, while it is open.
    pub branches: Option<Rect>,
}

/// The height **Repositories** takes while both sections are open, its heading included: as dragged, or where it
/// starts, [`REPOSITORIES_START`] or as tall as its rows when they need less; never less than [`SECTION_LEAST`], nor so
/// tall that **Branches** has less, while there is room for both.
#[must_use]
pub fn repositories_height(column: f32, sections: SideSections, (heading, rows): (f32, f32), line: f32) -> f32 {
    let start = REPOSITORIES_START.min(heading + rows);
    let room = (column - line).max(0.0);
    let most = (room - SECTION_LEAST).max(SECTION_LEAST);
    sections
        .repositories_height
        .unwrap_or(start)
        .clamp(SECTION_LEAST, most)
        .min(room)
}

/// Where the headings and the sections go in a column: a folded section is its heading alone, and the other takes the
/// height; **Branches** folded keeps its heading at the foot of the column.
#[must_use]
pub fn section_rects(column: Rect, sections: SideSections, (heading, rows): (f32, f32), line: f32) -> SectionRects {
    let mut area = Area::new(column);
    let repositories_heading = area.top(heading);
    match (sections.is_repositories_folded, sections.is_branches_folded) {
        (true, true) => SectionRects {
            repositories_heading,
            repositories: None,
            branches_heading: area.top(heading),
            branches: None,
        },
        (true, false) => {
            let branches_heading = area.top(heading);
            SectionRects {
                repositories_heading,
                repositories: None,
                branches_heading,
                branches: Some(area.rest()),
            }
        }
        (false, true) => {
            let branches_heading = area.bottom(heading);
            SectionRects {
                repositories_heading,
                repositories: Some(area.rest()),
                branches_heading,
                branches: None,
            }
        }
        (false, false) => {
            let height = repositories_height(column.height, sections, (heading, rows), line);
            let repositories = area.top((height - heading).max(0.0));
            let _line = area.top(line);
            let branches_heading = area.top(heading);
            SectionRects {
                repositories_heading,
                repositories: Some(repositories),
                branches_heading,
                branches: Some(area.rest()),
            }
        }
    }
}

/// The side column over the rectangle its caller clipped it to: the heading of **Repositories** with its chevron and
/// **+**, its rows, the line that is dragged between the sections while both are open, the heading of **Branches** and
/// the branch rail. A click on a heading folds or unfolds its section.
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
    let heading = theme.sizes.control_height;
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
            if !sections.is_repositories_folded && !sections.is_branches_folded {
                intents.extend(section_line(ui, column, sections, (heading, rows)));
            }
            let open = heading_row(ui, rects.repositories_heading, (SideSection::Repositories, sections));
            intents.extend(open);
            intents.extend(open_button(
                state.tabs.not_open,
                ui,
                Area::new(rects.repositories_heading).right(heading),
            ));
            if let Some(rows) = rects.repositories {
                intents.extend(ui.clip(rows, |ui| tabs_view(&state.tabs, ui)));
            }
            intents.extend(heading_row(
                ui,
                rects.branches_heading,
                (SideSection::Branches, sections),
            ));
            if let (Some(rail), Some(references)) = (rects.branches, &state.branches) {
                let (rail_intents, rail_hover) = ui.clip(rail, |ui| crate::branch::pane_view(references, ui));
                intents.extend(rail_intents);
                hover = rail_hover;
            }
        },
    );
    (intents, hover)
}

/// The line between the two sections, dragged or moved with the arrows to share the height; a double click puts
/// **Repositories** back where it starts.
fn section_line(ui: &mut Ui<'_>, column: Rect, sections: SideSections, (heading, rows): (f32, f32)) -> Option<Intent> {
    let line = ui.theme().sizes.border;
    let height = repositories_height(column.height, sections, (heading, rows), line);
    let splitter = Splitter {
        split: Split::Rows,
        default_size: REPOSITORIES_START.min(heading + rows),
        least_first: SECTION_LEAST,
        least_second: SECTION_LEAST,
        size: Some(height),
        step: STEP,
    };
    let response = ui.splitter_across("side sections line", column, &splitter, line);
    let rule = Rect::new(column.x, column.y + height, column.width, line);
    ui.separator(rule, Axis::Horizontal);
    let pane = |size| Intent::Graph(GraphIntent::SetPane(Pane::Repositories, size));
    if response.is_reset {
        Some(pane(None))
    } else if response.is_changed {
        Some(pane(Some(response.size)))
    } else {
        None
    }
}

/// The heading of a section: its chevron, down while it is open, and its name in capitals; a click folds or unfolds
/// it.
fn heading_row(ui: &mut Ui<'_>, rect: Rect, (section, sections): (SideSection, SideSections)) -> Option<Intent> {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let (name, is_folded) = match section {
        SideSection::Repositories => (words::REPOSITORIES, sections.is_repositories_folded),
        SideSection::Branches => (words::BRANCHES, sections.is_branches_folded),
    };
    let id = ui.id(("side heading", section));
    let response = ui.interact(id, rect, Sense::CLICK | Sense::FOCUS);
    if response.hovered {
        ui.fill_rect(rect, theme.colors.list_hover);
    }
    if response.shows_focus {
        ui.focus_ring(rect, theme.sizes.radius.medium);
    }
    let mut parts = Area::with_gap(inset(rect, Insets::symmetric(spacing.medium, 0.0)), spacing.small);
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
