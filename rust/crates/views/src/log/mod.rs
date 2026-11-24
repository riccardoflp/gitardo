//! The Command log, a tab of the bottom panel (`log.show`): what the log recorded, newest last, each line with the time it
//! started, following the log as it grows; the lines selected are copied with `Ctrl+C`.

use ui::layout::{Area, Insets, inset};
use ui::{
    Key, Label, List, Modifiers, Point, Rect, Reveal, RevealPlace, SelectionMode, Steering, TextKind, TextRun, Tone, Ui,
};

use crate::intent::Intent;
use crate::text::activity as words;

/// The key of the list of the lines.
const LINES: &str = "command log lines";

/// A line of the log, as the Command log shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogLine {
    /// When it started: `14:02:09`.
    pub time: String,
    /// The words: `git checkout -b fix/login HEAD`.
    pub text: String,
    /// Whether it is what git or the system said when the step above it failed.
    pub is_failure: bool,
    /// Whether it is what the step above it was given on its standard input, or what git printed under it once it
    /// succeeded, rather than a step: drawn muted.
    pub is_printed: bool,
}

/// What the Command log shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogState<'a> {
    /// The lines kept, oldest first.
    pub lines: &'a [LogLine],
    /// How many older lines were dropped to keep the last 10,000.
    pub dropped: usize,
    /// The line the log was opened at, scrolled to the top and marked; none to follow the last line.
    pub at: Option<LogAt>,
}

/// A line the Command log was opened at: the first line of the command that said what was clicked (`activity.said`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogAt {
    /// Its place among the lines kept.
    pub line: usize,
    /// A number that changes each time the log is opened at a line, so that it is scrolled to it once each time.
    pub serial: u64,
}

/// The Command log, over the rectangle its caller clipped it to: how many lines were dropped, when any were, then the
/// lines in a box, the last one brought into sight whenever one is added, or the line it was opened at brought to the
/// top and marked. Rows are selected with the pointer and the
/// keys of a list, and `Ctrl+C` copies them, each with its time.
///
/// Feature: log.show
pub fn log_view(state: &LogState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut area = Area::with_gap(ui.current_clip(), spacing.medium);
    if state.dropped > 0 {
        let said = words::dropped_lines(state.dropped);
        let label = Label::new(&said).kind(TextKind::Detail).tone(Tone::Muted);
        let _ = ui.label(area.top(theme.sizes.row_height), &label);
    }
    let box_rect = area.rest();
    ui.fill_rounded_rect(
        box_rect,
        theme.sizes.radius.large,
        theme.colors.secondary_button_background,
    );
    let inside = inset(box_rect, Insets::all(spacing.medium));
    if state.lines.is_empty() {
        let label = Label::new(words::LOG_EMPTY).kind(TextKind::Detail).tone(Tone::Muted);
        let _ = ui.label(
            Rect::new(inside.x, inside.y, inside.width, theme.sizes.row_height),
            &label,
        );
        return Vec::new();
    }
    lines_list(state, ui, inside);
    copy_selected(state, ui);
    Vec::new()
}

/// The lines, in a list that follows the last one, each with its time; what git or the system said in red, and what a
/// step was given or printed muted.
fn lines_list(state: &LogState<'_>, ui: &mut Ui<'_>, inside: Rect) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let style = ui.text_style(TextKind::Code);
    let list = List {
        name: words::COMMAND_LOG,
        row_count: state.lines.len(),
        row_height: ui.line_height(style) + spacing.tiny,
        selection_mode: SelectionMode::Multiple,
        lead: 0.0,
    };
    let copy = [(Key::Character('c'), Modifiers::CONTROL)];
    let steering = Steering {
        selection: None,
        reveal: Some(reveal(state)),
        left_to_view: &copy,
    };
    let marked = state.at.map(|at| at.line);
    let time_width = ui.measure("00:00:00", style) + spacing.medium;
    let _ = ui.steered_list(LINES, (inside, &list), &steering, |ui, row| {
        let Some(line) = state.lines.get(row.index) else {
            return;
        };
        if marked == Some(row.index) && !row.is_selected {
            ui.fill_rounded_rect(row.rect, theme.sizes.radius.small, theme.colors.list_inactive_selection);
        }
        let top = row.rect.y + spacing.tiny * 0.5;
        let _ = ui.text(&TextRun {
            text: &line.time,
            style,
            color: theme.colors.muted_foreground,
            origin: Point::new(row.rect.x + spacing.small, top),
            max_width: None,
        });
        let color = if line.is_failure {
            Tone::Error.color(&theme)
        } else if line.is_printed && !row.is_selected {
            theme.colors.muted_foreground
        } else {
            row.foreground
        };
        let _ = ui.text(&TextRun {
            text: &line.text,
            style,
            color,
            origin: Point::new(row.rect.x + spacing.small + time_width, top),
            max_width: Some(row.rect.width - time_width - spacing.small * 2.0),
        });
    });
}

/// The line brought into sight: the line the log was opened at, at the top, once each time it is opened so; else the
/// last line, each time one is added. Their numbers are odd and even, so that one never stands for the other.
fn reveal(state: &LogState<'_>) -> Reveal {
    match state.at {
        Some(at) => Reveal {
            serial: at.serial.saturating_mul(2).saturating_add(1),
            row: at.line,
            place: RevealPlace::Top,
        },
        None => Reveal {
            serial: u64::try_from(state.lines.len() + state.dropped)
                .unwrap_or(u64::MAX / 2)
                .saturating_mul(2),
            row: state.lines.len().saturating_sub(1),
            place: RevealPlace::Nearest,
        },
    }
}

/// Puts the lines selected on the clipboard when `Ctrl+C` was pressed in the list, each with its time.
fn copy_selected(state: &LogState<'_>, ui: &mut Ui<'_>) {
    let id = ui.id(LINES);
    let is_copied = ui
        .unclaimed_keys_of(id)
        .iter()
        .any(|key| key.modifiers == Modifiers::CONTROL && key.key == Key::Character('c'));
    if !is_copied {
        return;
    }
    let selection = ui.list_selection(LINES);
    let copied: Vec<String> = state
        .lines
        .iter()
        .enumerate()
        .filter(|(index, _)| selection.contains(*index))
        .map(|(_, line)| copied_line(line))
        .collect();
    if !copied.is_empty() {
        ui.copy_to_clipboard(copied.join("\n"));
    }
}

/// A line as it is copied: its time, then its words.
#[must_use]
pub fn copied_line(line: &LogLine) -> String {
    format!("{} {}", line.time, line.text)
}
