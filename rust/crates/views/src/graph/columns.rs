//! What is shown with each message (`graph.columns`): the author and the date on the line under it, after the labels of
//! the commit, the short hash at the end of the row, each as `graph.columns` asks.

use ui::layout::{Align, place};
use ui::{Point, Rect, Sense, TextKind, TextRun, Ui};

use super::intent::{Switches, TimelineSwitch};
use crate::intent::Intent;
use crate::text::dates::{full_date, relative_date};

/// The columns of a commit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColumnsState<'a> {
    /// The name git has for the author.
    pub author: &'a str,
    /// When the commit was made, in seconds since 1970.
    pub date: i64,
    /// The short hash.
    pub hash: &'a str,
    /// The switches of the timeline, the columns among them.
    pub switches: Switches,
    /// The moment the date is said from.
    pub now: i64,
    /// Whether the window is narrow, which leaves the hash out.
    pub is_narrow: bool,
    /// Where the line under the message starts, down.
    pub top: f32,
}

/// The width the hash takes at the end of a row: none when it is not shown.
pub(crate) fn hash_room(ui: &Ui<'_>, switches: Switches, is_narrow: bool) -> f32 {
    if is_narrow || !switches.is_on(TimelineSwitch::Hash) {
        return 0.0;
    }
    ui.measure("0000000", ui.text_style(TextKind::Hash))
}

/// How wide the author and the date are on the line under the message, as `columns_view` draws them.
pub(crate) fn author_and_date_width(ui: &Ui<'_>, state: &ColumnsState<'_>) -> f32 {
    let style = ui.text_style(TextKind::Detail);
    let mut width = 0.0;
    if state.switches.is_on(TimelineSwitch::Author) {
        width += ui.measure(state.author, style);
    }
    if state.switches.is_on(TimelineSwitch::Date) {
        width += ui.measure(&said_date(state), style);
    }
    width
}

/// The date as the line under the message says it: after a dot when the author is before it.
fn said_date(state: &ColumnsState<'_>) -> String {
    if state.switches.is_on(TimelineSwitch::Author) {
        format!(" \u{b7} {}", relative_date(state.date, state.now))
    } else {
        relative_date(state.date, state.now)
    }
}

/// The author and the date under the message, from the start of the rectangle its caller clipped it to, which is after
/// the labels of the commit, and the hash at the end of that rectangle. What is left out gives its room to the message.
///
/// Feature: graph.columns
pub fn columns_view(state: &ColumnsState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let muted = theme.colors.muted_foreground;
    let hash_width = hash_room(ui, state.switches, state.is_narrow);
    if hash_width > 0.0 {
        let style = ui.text_style(TextKind::Hash);
        let line = place(
            Rect::new(rect.right() - hash_width, rect.y, hash_width, rect.height),
            (hash_width, ui.line_height(style)),
            (Align::Start, Align::Center),
        );
        let run = TextRun {
            text: state.hash,
            style,
            color: muted,
            origin: Point::new(line.x, line.y),
            max_width: None,
        };
        let _ = ui.text(&run);
    }
    let style = ui.text_style(TextKind::Detail);
    let right = rect.right()
        - hash_width
        - if hash_width > 0.0 {
            theme.sizes.spacing.large
        } else {
            0.0
        };
    let mut x = rect.x;
    if state.switches.is_on(TimelineSwitch::Author) {
        let run = TextRun {
            text: state.author,
            style,
            color: muted,
            origin: Point::new(x, state.top),
            max_width: Some((right - x).max(0.0)),
        };
        x += ui.text(&run);
    }
    if state.switches.is_on(TimelineSwitch::Date) {
        let said = said_date(state);
        let run = TextRun {
            text: &said,
            style,
            color: muted,
            origin: Point::new(x, state.top),
            max_width: Some((right - x).max(0.0)),
        };
        let width = ui.text(&run);
        let date_rect = Rect::new(x, state.top, width, ui.line_height(style));
        let id = ui.id(("date", state.hash));
        let response = ui.interact(id, date_rect, Sense::HOVER);
        ui.tooltip(&response, &full_date(state.date));
    }
    Vec::new()
}
