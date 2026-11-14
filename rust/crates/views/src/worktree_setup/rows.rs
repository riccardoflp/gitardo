//! The rows of the page, one under the other in a scroll area: the header, what git ignores in sections, the patterns,
//! the commands and where they are saved. Each row is as high as a control, and only the rows in sight are drawn.

use model::worktree_setup::Section;
use ui::{Rect, Ui};

use super::lines::{SetupLine, is_open, lines};
use super::{SetupBox, SetupPageState, SetupRead};
use crate::intent::Intent;

/// The most lines a section lists before **Show the other `<n>`**.
pub(super) const SECTION_CAP: usize = 200;

/// A row of the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Row {
    /// **What new worktrees get**.
    Header,
    /// What a new worktree has and gets.
    HeaderDetail,
    /// **Linked now**, **Copied now** or **Run now**, by the box of its setting.
    Now(SetupBox),
    /// A row left empty between two parts.
    Gap,
    /// **What git ignores here**, **Use what is proposed** and **Look again**.
    IgnoredBar,
    /// What the page says of the read: looking, nothing ignored, changed since, or what git could not say.
    Note,
    /// The titles of the columns.
    Columns,
    /// The line of a section, which opens and closes it.
    Section(Section),
    /// A line of the table, by its place among the lines.
    Line(usize),
    /// A place of a name in several places: the line, and the place.
    Place(usize, usize),
    /// **Show the other `<n>`** of a section.
    ShowOthers(Section, usize),
    /// **Patterns**.
    PatternsTitle,
    /// The title of a list: the folders to link, the files to copy, or the commands.
    ListTitle(SetupBox),
    /// A pattern of a list, by its place in it.
    Pattern(SetupBox, usize),
    /// The box and **Add** of a list.
    Add(SetupBox),
    /// A command, by its place.
    Command(usize),
    /// **Proposed from what is committed here:** and a button for each proposal not in the list.
    Proposals,
    /// **Save in** and its two places.
    SaveIn,
    /// The box of VS Code's settings.
    AlsoVsCode,
    /// **Save**, **Discard changes**, **Bring into…**, **Remove from…**, **Edit in settings file**.
    Buttons,
}

/// What the rows are drawn from: the state of the page, and its lines.
pub(super) struct Drawn<'state, 'lines> {
    /// The state of the page.
    pub(super) state: &'state SetupPageState<'state>,
    /// The lines of the table.
    pub(super) lines: &'lines [SetupLine<'state>],
}

/// The rows of the page, in order.
pub(super) fn rows(state: &SetupPageState<'_>, all: &[SetupLine<'_>]) -> Vec<Row> {
    let mut rows = vec![
        Row::Header,
        Row::HeaderDetail,
        Row::Now(SetupBox::Link),
        Row::Now(SetupBox::Copy),
        Row::Now(SetupBox::Command),
        Row::Gap,
        Row::IgnoredBar,
    ];
    if has_note(state) {
        rows.push(Row::Note);
    }
    if !all.is_empty() {
        rows.push(Row::Columns);
    }
    for section in Section::ALL {
        rows.extend(section_rows(state, all, section));
    }
    rows.extend([Row::Gap, Row::PatternsTitle, Row::ListTitle(SetupBox::Link)]);
    let patterns = state.chosen.patterns;
    rows.extend((0..patterns.link.len()).map(|index| Row::Pattern(SetupBox::Link, index)));
    rows.extend([Row::Add(SetupBox::Link), Row::ListTitle(SetupBox::Copy)]);
    rows.extend((0..patterns.copy.len()).map(|index| Row::Pattern(SetupBox::Copy, index)));
    rows.extend([Row::Add(SetupBox::Copy), Row::Gap, Row::ListTitle(SetupBox::Command)]);
    rows.extend((0..state.commands.len()).map(Row::Command));
    rows.push(Row::Add(SetupBox::Command));
    if state
        .proposals
        .iter()
        .any(|proposal| !state.commands.contains(&proposal.command))
    {
        rows.push(Row::Proposals);
    }
    rows.extend([Row::Gap, Row::SaveIn]);
    if state.place == model::worktree_setup::SettingsPlace::Repository {
        rows.push(Row::AlsoVsCode);
    }
    rows.push(Row::Buttons);
    rows
}

/// Whether the page says something of the read under its bar.
fn has_note(state: &SetupPageState<'_>) -> bool {
    match state.read {
        SetupRead::Looking | SetupRead::Failed(_) => true,
        SetupRead::Read(entries) => {
            entries.is_empty() || state.look_failed.is_some() || state.chosen.patterns != state.chosen.looked
        }
    }
}

/// The rows of a section: its line, then, while it is open, its lines with the places of the names unfolded, at most
/// [`SECTION_CAP`] of them unless all are shown.
fn section_rows(state: &SetupPageState<'_>, all: &[SetupLine<'_>], section: Section) -> Vec<Row> {
    let of_section: Vec<usize> = (0..all.len())
        .filter(|index| all.get(*index).is_some_and(|line| line.section == section))
        .collect();
    if of_section.is_empty() {
        return Vec::new();
    }
    let has_something = of_section.iter().filter_map(|index| all.get(*index)).any(|line| {
        line.places
            .iter()
            .any(|place| state.chosen.place_gets(place) != model::worktree_setup::Gets::Nothing)
    });
    let mut rows = vec![Row::Section(section)];
    if !is_open(section, state.toggled, has_something) {
        return rows;
    }
    let shown = if state.shown_all.contains(&section) {
        of_section.len()
    } else {
        SECTION_CAP.min(of_section.len())
    };
    for index in of_section.iter().take(shown) {
        rows.push(Row::Line(*index));
        let Some(line) = all.get(*index) else {
            continue;
        };
        if line.is_name() && state.unfolded.contains(&line.pattern) {
            rows.extend((0..line.places.len()).map(|place| Row::Place(*index, place)));
        }
    }
    if shown < of_section.len() {
        rows.push(Row::ShowOthers(section, of_section.len() - shown));
    }
    rows
}

/// The height of every row.
pub(super) fn row_height(ui: &Ui<'_>) -> f32 {
    let sizes = ui.theme().sizes;
    sizes.control_height + sizes.spacing.small
}

/// A count of rows as a length.
fn count(rows: usize) -> f32 {
    f32::from(u16::try_from(rows).unwrap_or(u16::MAX))
}

/// The rows of the page in a scroll area over a rectangle, those in sight drawn; what the user asked.
pub(super) fn draw(state: &SetupPageState<'_>, ui: &mut Ui<'_>, area: Rect) -> Vec<Intent> {
    let entries = match state.read {
        SetupRead::Read(entries) => entries,
        SetupRead::Looking | SetupRead::Failed(_) => &[],
    };
    let all = lines(entries);
    let rows = rows(state, &all);
    let height = row_height(ui);
    let drawn = Drawn { state, lines: &all };
    let content = (area.width, height * count(rows.len()));
    let (_, intents) = ui.scroll_area("new worktrees rows", area, content, |ui, inside| {
        let mut intents = Vec::new();
        for (index, row) in rows.iter().enumerate() {
            let top = inside.y + height * count(index);
            if top + height < area.y || top > area.bottom() {
                continue;
            }
            let gutter = ui.theme().sizes.spacing.large * 1.5;
            let rect = Rect::new(inside.x, top, (inside.width - gutter).max(0.0), height);
            intents.extend(draw_row(&drawn, *row, ui, rect));
        }
        intents
    });
    intents
}

/// One row.
fn draw_row(drawn: &Drawn<'_, '_>, row: Row, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    match row {
        Row::Header | Row::HeaderDetail | Row::Now(_) | Row::IgnoredBar | Row::Note | Row::Columns => {
            super::table::head_row(drawn, row, ui, rect)
        }
        Row::Section(section) => super::table::section_row(drawn, section, ui, rect),
        Row::Line(index) => super::table::line_row(drawn, index, ui, rect),
        Row::Place(index, place) => super::table::place_row(drawn, (index, place), ui, rect),
        Row::ShowOthers(section, others) => super::table::show_others(section, others, ui, rect),
        Row::PatternsTitle | Row::ListTitle(_) | Row::Pattern(..) | Row::Add(_) | Row::Command(_) | Row::Proposals => {
            super::lists::list_row(drawn.state, row, ui, rect)
        }
        Row::SaveIn | Row::AlsoVsCode | Row::Buttons => super::bottom::bottom_row(drawn.state, row, ui, rect),
        Row::Gap => Vec::new(),
    }
}
