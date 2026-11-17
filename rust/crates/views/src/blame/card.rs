//! The hover of the blame of a committed line: the author, the date and time and the summary of its commit, and the
//! links that show it in the timeline, open it as a page and, when the commit has one before it for the file, blame the
//! file as it was before (`blame.file`, `blame.current-line`).

use std::time::Duration;

use model::blame::BlameLine;
use model::contract::FeatureId;
use ui::layout::{Area, Insets, inset};
use ui::{Button, ButtonKind, Label, Popover, Rect, TextKind, Tone, Ui};

use super::BlameIntent;
use crate::file_view::FilePageId;
use crate::intent::Intent;
use crate::text::blame as words;

/// How long the pointer rests on the blame of a line before its hover opens.
pub(super) const REST: Duration = Duration::from_millis(400);

/// How wide the hover is.
const WIDTH: f32 = 380.0;

/// The hover of the blame of a line, from 0, under the rectangle of what is beside it: in the column of the blame of the
/// file, with **Open it as a page** and **Blame the file as it was before** under **Show `<short hash>` in the
/// timeline**; at the end of the line of the cursor, with that link alone. Gives what a link asks, and that the hover
/// closes once the pointer leaves both.
pub(super) fn hover_card(
    ui: &mut Ui<'_>,
    (page, line, is_in_column): (FilePageId, usize, bool),
    blamed: &BlameLine,
    anchor: Rect,
) -> Vec<Intent> {
    let BlameLine::Committed(commit) = blamed else {
        return Vec::new();
    };
    let theme = *ui.theme();
    let row = theme.sizes.row_height;
    let links = match (is_in_column, commit.previous.is_some()) {
        (false, _) => 1.0,
        (true, false) => 2.0,
        (true, true) => 3.0,
    };
    let popover = Popover {
        anchor,
        size: (WIDTH, row * (3.0 + links) + 2.0 * theme.sizes.spacing.large),
        name: words::HOVER,
        side: ui::PopoverSide::Below,
    };
    let date = crate::text::dates::full_date(commit.author_time.0);
    let short = commit.id.short();
    let shown = words::show_in_timeline(short);
    let answer = ui.popover(("blame hover", page, line), &popover, |ui, rect, _| {
        let mut area = Area::new(inset(rect, Insets::all(theme.sizes.spacing.large)));
        let _ = ui.label(area.top(row), &Label::new(&commit.author).kind(TextKind::Heading));
        let _ = ui.label(area.top(row), &Label::new(&date).tone(Tone::Muted));
        let _ = ui.label(area.top(row), &Label::new(&commit.summary));
        let mut asked = None;
        let mut add_link = |ui: &mut Ui<'_>, said: &str, feature: &'static str| {
            let button = Button::secondary(said).kind(ButtonKind::Icon);
            if ui.button(("blame hover link", feature), area.top(row), &button).clicked {
                asked = Some(feature);
            }
        };
        add_link(ui, &shown, "blame.reveal-line-commit");
        if is_in_column {
            add_link(ui, words::OPEN_AS_PAGE, "blame.open-line-commit");
        }
        if is_in_column && commit.previous.is_some() {
            add_link(ui, words::BLAME_FILE_BEFORE, "blame.previous");
        }
        asked
    });
    let mut intents = Vec::new();
    if let Some(feature) = answer.inner {
        intents.push(Intent::Blame(BlameIntent::Run(
            page,
            FeatureId(feature),
            Some(line + 1),
        )));
    }
    let is_left = ui
        .pointer()
        .is_none_or(|point| !answer.rect.contains(point) && !anchor.contains(point));
    if answer.inner.is_some() || answer.is_dismissed || (is_left && !answer.is_new) {
        intents.push(Intent::Blame(BlameIntent::Hover(page, None)));
    }
    intents
}
