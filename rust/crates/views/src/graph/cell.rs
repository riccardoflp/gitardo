//! The cell of the graph of a row (`graph.lanes`): the lines through it and its dot, a dot for a commit, a ring for a
//! merge, a dashed ring for the draft and a fold, a square for a stash, and the ring of a commit not pushed
//! (`graph.unpushed`); what is not in the history brought out fades (`graph.trace-history`), and so does the rest of
//! its row (`graph.hover-history`, `graph.trace-selected`).

use model::graph::{CellLine, CellTrace, GraphCell, Half};
use ui::{Color, Point, Rect, Stroke, Ui};

use super::state::TraceStrength;

/// The room left at each side of the lanes.
const MARGIN: f32 = 3.0;
/// The dashes and gaps of the line of the draft and of a stash.
const DASH: (f32, f32) = (3.0, 3.0);
/// How many dashes the ring of the draft and of a fold has.
const RING_DASHES: u8 = 8;
/// How opaque what fades under a label is, out of 255.
const FADED_UNDER_LABEL: u8 = 50;
/// How opaque what is dimmed by the selected commit is, out of 255.
const FADED_BY_SELECTION: u8 = 110;
/// How opaque the message, labels, author, date and hash of a row left out under a label are, out of 255: about a
/// third, as the extension does.
const ROW_FADED_UNDER_LABEL: u8 = 89;
/// How opaque they are when the selected commit leaves the row out: about two thirds, as the extension does.
const ROW_FADED_BY_SELECTION: u8 = 153;

/// What the dot of a cell is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dot {
    /// A commit.
    Commit,
    /// A merge: a ring.
    Merge,
    /// A stash: a square with no fill.
    Stash,
    /// The draft or a fold: a ring in dashes.
    Dashed,
}

/// Where a cell is drawn and how.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CellPlace {
    /// The rectangle of the graph in the row.
    pub rect: Rect,
    /// The width of each lane.
    pub lane_width: f32,
    /// What the dot is.
    pub dot: Dot,
    /// Whether the commit is not pushed: a ring around its dot.
    pub is_unpushed: bool,
}

/// The middle of a lane, across.
pub(crate) fn lane_x(rect: Rect, lane_width: f32, lane: usize) -> f32 {
    rect.x + MARGIN + lane_width * (count(lane) + 0.5)
}

/// A count as a length.
#[allow(
    clippy::cast_precision_loss,
    reason = "lanes are counted in tens, which an f32 holds exactly"
)]
fn count(lane: usize) -> f32 {
    lane as f32
}

/// The color of a line or a dot, faded when it is not in the history brought out.
fn faded(color: Color, is_in: bool, strength: Option<TraceStrength>) -> Color {
    match (is_in, strength) {
        (false, Some(TraceStrength::Label)) => color.with_alpha(FADED_UNDER_LABEL),
        (false, Some(TraceStrength::Selected)) => color.with_alpha(FADED_BY_SELECTION),
        _ => color,
    }
}

/// How opaque the rest of a row is, out of 255: a row the history brought out leaves out steps back whole, more under
/// a label than by the selected commit; its background and its selection stay as they are.
pub(crate) fn row_opacity(trace: Option<(&CellTrace, TraceStrength)>) -> u8 {
    match trace {
        Some((cells, TraceStrength::Label)) if !cells.is_in => ROW_FADED_UNDER_LABEL,
        Some((cells, TraceStrength::Selected)) if !cells.is_in => ROW_FADED_BY_SELECTION,
        _ => u8::MAX,
    }
}

/// Draws a cell: its lines first, then its dot over them.
pub(crate) fn draw_cell(
    ui: &mut Ui<'_>,
    cell: &GraphCell,
    place: &CellPlace,
    trace: Option<(&CellTrace, TraceStrength)>,
) {
    let theme = *ui.theme();
    let rect = place.rect;
    let middle = rect.y + rect.height * 0.5;
    let strength = trace.map(|(_, strength)| strength);
    for (index, line) in cell.lines.iter().enumerate() {
        let is_in = trace.is_none_or(|(cells, _)| cells.lines_in.get(index).copied().unwrap_or(false));
        let color = faded(theme.lane(line.color), is_in, strength);
        draw_line(ui, line, (rect, place.lane_width, middle), color);
    }
    let is_in = trace.is_none_or(|(cells, _)| cells.is_in);
    let color = faded(theme.lane(cell.color), is_in, strength);
    let center = Point::new(lane_x(rect, place.lane_width, cell.lane), middle);
    let radius = theme.sizes.graph_dot.min(place.lane_width * 0.45).max(2.0);
    draw_dot(ui, place.dot, (center, radius), color);
    if place.is_unpushed {
        let stroke = Stroke {
            width: theme.sizes.border,
            cap: ui::LineCap::Round,
            color,
        };
        ui.stroke_circle(center, radius + theme.sizes.spacing.tiny + theme.sizes.border, stroke);
    }
}

/// Draws one line of a cell, in its half of the row.
fn draw_line(ui: &mut Ui<'_>, line: &CellLine, (rect, lane_width, middle): (Rect, f32, f32), color: Color) {
    let (top, bottom) = match line.half {
        Half::Upper => (rect.y, middle),
        Half::Lower => (middle, rect.bottom()),
    };
    let from = Point::new(lane_x(rect, lane_width, line.from), top);
    let to = Point::new(lane_x(rect, lane_width, line.to), bottom);
    let stroke = Stroke {
        width: ui.theme().sizes.graph_line,
        cap: ui::LineCap::Butt,
        color,
    };
    if line.is_dashed {
        ui.dashed_lane_line(from, to, stroke, DASH);
    } else {
        ui.lane_line(from, to, stroke);
    }
}

/// Draws the dot of a cell, on a ring of the ground so that the lines stop short of it.
fn draw_dot(ui: &mut Ui<'_>, dot: Dot, (center, radius): (Point, f32), color: Color) {
    let theme = *ui.theme();
    let ground = theme.colors.page_background;
    let line = theme.sizes.graph_line;
    let stroke = Stroke {
        width: line,
        cap: ui::LineCap::Butt,
        color,
    };
    match dot {
        Dot::Commit => {
            ui.fill_circle(center, radius + theme.sizes.spacing.tiny, ground);
            ui.fill_circle(center, radius, color);
        }
        Dot::Merge => {
            ui.fill_circle(center, radius + theme.sizes.spacing.tiny, ground);
            ui.fill_circle(center, radius, color);
            ui.fill_circle(center, (radius - line).max(0.0), ground);
        }
        Dot::Stash => {
            let side = radius * 2.0;
            let square = Rect::new(center.x - radius, center.y - radius, side, side);
            ui.fill_rect(square, ground);
            ui.border(
                square,
                ui::Border {
                    width: line,
                    radius: 1.0,
                    color,
                },
            );
        }
        Dot::Dashed => {
            ui.fill_circle(center, radius + theme.sizes.spacing.tiny, ground);
            ui.dashed_ring(center, radius - line * 0.5, (stroke, RING_DASHES));
        }
    }
}
