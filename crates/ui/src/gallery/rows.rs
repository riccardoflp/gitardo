//! The right pane of the gallery: a list of a million commits over a box of labels, badges and icons beside a scroll
//! area, the two split by a line that can be dragged.

use std::hash::Hasher;

use render::{Icon, Point, Rect, TextRun};

use super::overlays::{self, ROW_MENU};
use super::{GalleryIntent, GalleryState, ROW_COUNT};
use crate::frame::{TextKind, Ui};
use crate::id::StableHasher;
use crate::layout::{Align, Area, Insets, Size, columns, inset, place};
use crate::widget::Tone;
use crate::widget::badge::Badge;
use crate::widget::find::ListSearch;
use crate::widget::label::Label;
use crate::widget::list::{List, Row, SelectionMode};
use crate::widget::panel::Axis;
use crate::widget::splitter::{Split, Splitter};

/// The messages the rows of the list take in turn.
const MESSAGES: [&str; 12] = [
    "Fix the parser of the configuration file when a section has no name",
    "Add the login page",
    "Show the branches checked out last above the others",
    "Read the repository again when a file changes",
    "Merge branch 'feature/search' into main",
    "Keep the selection when the list is read again",
    "Draw the lines of the graph with the colors of the theme",
    "Remove the old settings page",
    "Update the README",
    "Cancel a fetch that takes too long",
    "Speed up the reading of large histories",
    "Rename the command palette entries",
];

/// The badges of the box of labels, which can be dragged onto the list.
const DRAGGED_BADGES: [&str; 2] = ["main", "v2.4.0"];

/// The message of a row.
fn message(row: usize) -> &'static str {
    MESSAGES.get(row % MESSAGES.len()).copied().unwrap_or_default()
}

/// The text find in page sees in a row: its hash and its message.
fn row_text(row: usize) -> String {
    format!("{} {}", short_hash(row), message(row))
}

/// The short hash shown for a row: seven hexadecimal digits made from its number.
pub(super) fn short_hash(row: usize) -> String {
    let mut hasher = StableHasher::new();
    hasher.write_usize(row);
    format!("{:07x}", hasher.finish() & 0x0FFF_FFFF)
}

/// Declares the right pane.
pub(super) fn rows(state: &GalleryState, ui: &mut Ui<'_>, pane: Rect, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    let split = Splitter {
        split: Split::Rows,
        default_size: (pane.height - theme.sizes.row_height * 12.0).max(0.0),
        least_first: theme.sizes.row_height * 4.0,
        least_second: theme.sizes.row_height * 6.0,
    };
    let panes = ui.splitter("rows", pane, &split);
    commits(state, ui, panes.first, intents);
    let bottom = inset(panes.second, Insets::all(theme.sizes.spacing.large));
    let [labels, scrolled] = columns(bottom, theme.sizes.spacing.large, [Size::Fill(1.2), Size::Fill(1.0)]);
    labels_panel(ui, labels);
    scroll_panel(ui, scrolled);
}

/// The list of a million commits, which find in page searches all of, with its context menu, and on which the
/// badges of the box of labels can be dropped.
fn commits(state: &GalleryState, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<GalleryIntent>) {
    let list = List {
        row_count: ROW_COUNT,
        row_height: ui.theme().sizes.row_height,
        selection_mode: SelectionMode::Multiple,
    };
    let search = ListSearch {
        row_text: &row_text,
        generation: 0,
    };
    let answer = ui.searchable_list("commits", (rect, &list), &search, commit_row);
    if let Some(row) = answer.activated {
        intents.push(GalleryIntent::Opened(row));
    }
    if let Some(row) = answer.context_row {
        intents.push(GalleryIntent::MenuOn(row));
    }
    let resets = overlays::reset_menu();
    let entries = overlays::row_menu(&resets);
    if let Some(command) = ui.context_menu(ROW_MENU, &answer.response, &entries) {
        overlays::run_command(ui, command, answer.context_row.or(state.menu_row), intents);
    }
    if let Some(source) = ui.drop_target(rect).dropped {
        let dropped = DRAGGED_BADGES
            .into_iter()
            .find(|text| ui.id(("badge", *text)) == source);
        if let Some(text) = dropped {
            intents.push(GalleryIntent::Did(format!("Dropped {text} on the list")));
        }
    }
}

/// What is in a row of the list: the hash, the message, and a badge on some rows.
fn commit_row(ui: &mut Ui<'_>, row: &Row) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let inner = inset(row.rect, Insets::symmetric(spacing.medium, 0.0));
    let mut area = Area::with_gap(inner, spacing.medium);
    let hash_width = ui.measure("0000000", ui.text_style(TextKind::Code));
    let hash = area.left(hash_width);
    let tone = if row.is_selected { Tone::Normal } else { Tone::Muted };
    let hash_text = short_hash(row.index);
    let code = Label::new(&hash_text).kind(TextKind::Code).tone(tone);
    let _ = ui.label(hash, &code);
    let badge = match row.index % 7 {
        0 => Some(Badge {
            text: "main",
            icon: Some(Icon::Branch),
            tone: Tone::Accent,
        }),
        3 => Some(Badge {
            text: "v2.4.0",
            icon: Some(Icon::Tag),
            tone: Tone::Warning,
        }),
        5 => Some(Badge {
            text: "origin/main",
            icon: Some(Icon::Cloud),
            tone: Tone::Normal,
        }),
        _ => None,
    };
    if let Some(badge) = badge {
        let width = ui.badge_width(&badge);
        let _ = ui.badge(area.right(width), &badge);
    }
    let text = area.rest();
    let style = ui.text_style(TextKind::Body);
    let line = place(text, (text.width, ui.line_height(style)), (Align::Start, Align::Center));
    let run = TextRun {
        text: message(row.index),
        style,
        color: row.foreground,
        origin: Point::new(line.x, line.y),
        max_width: Some(text.width),
    };
    let _ = ui.text(&run);
}

/// Labels of each tone, a label cut with an ellipsis, badges, icons and a separator.
fn labels_panel(ui: &mut Ui<'_>, rect: Rect) {
    let theme = *ui.theme();
    let inside = ui.panel(rect, Some("Labels and badges"));
    let row_height = theme.sizes.row_height;
    let mut rows = Area::with_gap(inside, theme.sizes.spacing.small);
    let _ = ui.heading(rows.top(row_height + theme.sizes.spacing.medium), "Heading");
    let [plain, muted] = columns(
        rows.top(row_height),
        theme.sizes.spacing.medium,
        [Size::Fill(1.0), Size::Fill(1.0)],
    );
    let _ = ui.label(plain, &Label::new("Plain text"));
    let _ = ui.label(muted, &Label::new("Muted text").tone(Tone::Muted));
    let long = "A label too long for its place is cut with an ellipsis, and shows all of it in a tooltip";
    let _ = ui.label(rows.top(row_height), &Label::new(long));
    ui.separator(rows.top(theme.sizes.spacing.small), Axis::Horizontal);
    let mut badges = Area::with_gap(rows.top(row_height), theme.sizes.spacing.small);
    let shown = [
        Badge {
            text: "main",
            icon: Some(Icon::Branch),
            tone: Tone::Accent,
        },
        Badge {
            text: "v2.4.0",
            icon: Some(Icon::Tag),
            tone: Tone::Warning,
        },
        Badge {
            text: "fixed",
            icon: None,
            tone: Tone::Success,
        },
        Badge {
            text: "12",
            icon: None,
            tone: Tone::Normal,
        },
    ];
    for badge in &shown {
        let width = ui.badge_width(badge);
        let drawn = ui.badge(badges.left(width), badge);
        if DRAGGED_BADGES.contains(&badge.text) {
            let _ = ui.drag_source(("badge", badge.text), drawn, badge.text);
        }
    }
    let mut icons = Area::with_gap(rows.top(row_height), theme.sizes.spacing.small);
    let tones = [
        (Icon::Commit, Tone::Normal),
        (Icon::Folder, Tone::Muted),
        (Icon::File, Tone::Muted),
        (Icon::Worktree, Tone::Accent),
        (Icon::Stash, Tone::Normal),
        (Icon::Info, Tone::Accent),
        (Icon::Warning, Tone::Warning),
        (Icon::Error, Tone::Error),
        (Icon::Check, Tone::Success),
    ];
    for (icon, tone) in tones {
        ui.icon_in(icons.left(theme.sizes.icon), icon, tone);
    }
}

/// A scroll area over lines of a command log, longer and wider than its box.
fn scroll_panel(ui: &mut Ui<'_>, rect: Rect) {
    let theme = *ui.theme();
    let inside = ui.panel(rect, Some("Scroll area"));
    let style = ui.text_style(TextKind::Code);
    let line = ui.line_height(style);
    let commands = [
        "git fetch --prune origin",
        "git switch --create feature/login-page origin/main",
        "git commit --file - --cleanup=strip",
        "git push --set-upstream origin feature/login-page --force-with-lease",
    ];
    let count = 40_usize;
    let content_height = crate::number::float_of_count(count) * line;
    let content_width = inside.width * 1.6;
    let color = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let _ = ui.scroll_area("log", inside, (content_width, content_height), |ui, content| {
        for index in 0..count {
            let y = content.y + crate::number::float_of_count(index) * line;
            if y + line < inside.y || y > inside.bottom() {
                continue;
            }
            let command = commands.get(index % commands.len()).copied().unwrap_or_default();
            let number = format!("{:>3}", index + 1);
            let digits = ui.text(&TextRun {
                text: &number,
                style,
                color: muted,
                origin: Point::new(content.x, y),
                max_width: None,
            });
            let origin = Point::new(content.x + digits + theme.sizes.spacing.medium, y);
            let _ = ui.text(&TextRun {
                text: command,
                style,
                color,
                origin,
                max_width: None,
            });
        }
    });
}
