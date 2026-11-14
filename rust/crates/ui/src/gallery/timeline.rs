//! The timeline, in the middle of the window: the draft of the next commit in a card with a dashed edge, over the
//! history of a million commits, each a row of two lines beside the graph — the subject with the references on the
//! commit, then who wrote it and when — with its hash on the right.

use std::f32::consts::{FRAC_PI_2, PI};

use render::{Color, LineCap, Point, Rect, Stroke, TextRun};

use super::graph::LEAD;
use super::menus::{self, ROW_MENU};
use super::{Field, GalleryIntent, GalleryState, Overlay, ROW_COUNT, data};
use crate::access::AccessRole;
use crate::frame::{TextKind, Ui};
use crate::layout::{Align, Area, Insets, inset, place};
use crate::widget::Tone;
use crate::widget::badge::{Badge, BadgeFill};
use crate::widget::button::Button;
use crate::widget::find::ListSearch;
use crate::widget::input_box::InputBox;
use crate::widget::list::{List, Row, Selection, SelectionMode};
use crate::widget::progress::arc_path;

/// The height of the card of the draft.
const DRAFT_HEIGHT: f32 = 76.0;
/// The side of the dashed ring of the draft, where its commit will be.
const DRAFT_RING: f32 = 14.0;
/// The length of a dash of the edge of the draft, and of the gap after it.
const DASH: (f32, f32) = (6.0, 4.0);
/// How many dashes the ring of the draft has.
const RING_DASHES: u8 = 8;

/// Declares the timeline over the middle of the window.
pub(super) fn timeline(state: &GalleryState, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let padding = spacing.large + spacing.small;
    let inner = inset(
        rect,
        Insets {
            left: padding,
            top: padding,
            right: padding,
            bottom: 0.0,
        },
    );
    let mut area = Area::with_gap(inner, spacing.large);
    let card = area.top(DRAFT_HEIGHT);
    ui.region(
        "draft region",
        (card, AccessRole::Region, "Uncommitted changes"),
        |ui| {
            draft(state, ui, card, intents);
        },
    );
    history(state, ui, area.rest(), intents);
}

/// The card of the draft: the message of the next commit, what changed, and Review and Commit.
fn draft(state: &GalleryState, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let radius = theme.sizes.radius.large + spacing.small;
    let edge = theme.lane(1);
    ui.fill_rounded_rect(rect, radius, theme.colors.card_background);
    dashed_border(ui, rect, radius, edge);
    let inner = inset(rect, Insets::symmetric(spacing.large + spacing.tiny, spacing.large));
    let mut area = Area::with_gap(inner, spacing.large + spacing.tiny);
    let ring = area.left(DRAFT_RING);
    dashed_ring(ui, ring.center(), edge);
    let commit = Button::primary("Commit");
    let review = Button::outline("Review");
    let (commit_width, height) = ui.button_size(&commit);
    let commit_rect = place(
        area.right(commit_width),
        (commit_width, height),
        (Align::Start, Align::Center),
    );
    let (review_width, _) = ui.button_size(&review);
    let review_rect = place(
        area.right(review_width),
        (review_width, height),
        (Align::Start, Align::Center),
    );
    let mut lines = Area::with_gap(area.rest(), spacing.small);
    let input = InputBox::new(&state.message, "Describe the next commit");
    let answer = ui.input_box("draft message", lines.top(height), &input);
    if let Some(text) = answer.changed_text {
        intents.push(GalleryIntent::Typed(Field::Message, text));
    }
    changes_line(ui, lines.rest());
    if ui.button("review", review_rect, &review).clicked {
        intents.push(GalleryIntent::Clicked(String::from("Review")));
    }
    if ui.button("commit", commit_rect, &commit).clicked || answer.is_submitted {
        let message = state.message.trim();
        intents.push(if message.is_empty() {
            GalleryIntent::Open(Overlay::EmptyMessage)
        } else {
            GalleryIntent::Did(format!("Committed \u{201c}{message}\u{201d}"))
        });
    }
}

/// What the draft changes: how many files, how many staged, and the lines added and removed in their colors.
fn changes_line(ui: &mut Ui<'_>, rect: Rect) {
    let theme = *ui.theme();
    let style = ui.text_style(TextKind::Detail);
    let line = place(rect, (rect.width, ui.line_height(style)), (Align::Start, Align::Center));
    let parts = [
        ("3 files changed \u{b7} 2 staged \u{b7} ", theme.colors.muted_foreground),
        ("+61", theme.colors.added),
        (" \u{2212}7", theme.colors.removed),
    ];
    let mut x = line.x;
    for (text, color) in parts {
        let run = TextRun {
            text,
            style,
            color,
            origin: Point::new(x, line.y),
            max_width: Some((line.right() - x).max(0.0)),
        };
        x += ui.text(&run);
    }
}

/// Draws the edge of a card in dashes along its sides, with its rounded corners whole.
fn dashed_border(ui: &mut Ui<'_>, rect: Rect, radius: f32, color: Color) {
    let width = ui.theme().sizes.border;
    let stroke = Stroke {
        width,
        cap: LineCap::Butt,
        color,
    };
    let half = width * 0.5;
    let corners = [
        (Point::new(rect.x + radius, rect.y + radius), PI),
        (Point::new(rect.right() - radius, rect.y + radius), PI + FRAC_PI_2),
        (Point::new(rect.right() - radius, rect.bottom() - radius), 0.0),
        (Point::new(rect.x + radius, rect.bottom() - radius), FRAC_PI_2),
    ];
    for (center, start) in corners {
        ui.stroke_path(arc_path(center, radius - half, start, FRAC_PI_2), stroke);
    }
    let (dash, gap) = DASH;
    let mut along = rect.x + radius;
    while along < rect.right() - radius {
        let length = dash.min(rect.right() - radius - along);
        ui.fill_rect(Rect::new(along, rect.y, length, width), color);
        ui.fill_rect(Rect::new(along, rect.bottom() - width, length, width), color);
        along += dash + gap;
    }
    let mut down = rect.y + radius;
    while down < rect.bottom() - radius {
        let length = dash.min(rect.bottom() - radius - down);
        ui.fill_rect(Rect::new(rect.x, down, width, length), color);
        ui.fill_rect(Rect::new(rect.right() - width, down, width, length), color);
        down += dash + gap;
    }
}

/// Draws the dashed ring of the draft, where its commit will be in the graph.
fn dashed_ring(ui: &mut Ui<'_>, center: Point, color: Color) {
    let line = ui.theme().sizes.graph_line;
    let stroke = Stroke {
        width: line,
        cap: LineCap::Butt,
        color,
    };
    let radius = (DRAFT_RING - line) * 0.5;
    let step = 2.0 * PI / f32::from(RING_DASHES);
    for dash in 0..RING_DASHES {
        let start = f32::from(dash) * step;
        ui.stroke_path(arc_path(center, radius, start, step * 0.6), stroke);
    }
}

/// The history: a list of a million commits that find in page searches all of, with its context menu, and on which
/// the references of the commits can be dropped.
fn history(state: &GalleryState, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<GalleryIntent>) {
    let list = List {
        name: "History",
        row_count: ROW_COUNT,
        row_height: ui.theme().sizes.timeline_row_height,
        selection_mode: SelectionMode::Multiple,
        lead: LEAD,
    };
    let search = ListSearch {
        row_text: &data::row_text,
        generation: 0,
    };
    let answer = ui.searchable_list("history", (rect, &list), &search, commit_row);
    if let Some(row) = answer.activated {
        intents.push(GalleryIntent::Opened(row));
    }
    if let Some(row) = answer.changed_selection.as_ref().and_then(Selection::cursor) {
        intents.push(GalleryIntent::Picked(row));
    }
    if let Some(row) = answer.context_row {
        intents.push(GalleryIntent::MenuOn(row));
    }
    let resets = menus::reset_menu();
    let entries = menus::row_menu(&resets);
    if let Some(command) = ui.context_menu(ROW_MENU, &answer.response, &entries) {
        menus::run_command(ui, command, answer.context_row.or(state.menu_row), intents);
    }
    if let Some(source) = ui.drop_target(rect).dropped {
        let dropped = (0..data::BLOCK)
            .flat_map(data::references)
            .find(|reference| ui.id(("reference", reference.name)) == source);
        if let Some(reference) = dropped {
            intents.push(GalleryIntent::Did(format!(
                "Dropped {} on the timeline",
                reference.name
            )));
        }
    }
}

/// A row of the history: its part of the graph, the subject with the references on the commit, who wrote it and
/// when, and the hash.
fn commit_row(ui: &mut Ui<'_>, row: &Row) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    ui.graph_cell(row.index, row.rect);
    let content = Rect::new(
        row.rect.x + LEAD + spacing.large,
        row.rect.y,
        (row.rect.width - LEAD - 2.0 * spacing.large - theme.sizes.spacing.small).max(0.0),
        row.rect.height,
    );
    let mut area = Area::with_gap(content, spacing.large);
    let hash_style = ui.text_style(TextKind::Hash);
    let hash = data::short_hash(row.index);
    let hash_width = ui.measure("0000000", hash_style);
    let hash_rect = area.right(hash_width);
    let hash_line = place(
        hash_rect,
        (hash_width, ui.line_height(hash_style)),
        (Align::Start, Align::Center),
    );
    let run = TextRun {
        text: &hash,
        style: hash_style,
        color: theme.colors.muted_foreground,
        origin: Point::new(hash_line.x, hash_line.y),
        max_width: None,
    };
    let _ = ui.text(&run);
    two_lines(ui, row, area.rest());
}

/// The two lines of a row: the subject with the references after it, then who wrote the commit and when.
fn two_lines(ui: &mut Ui<'_>, row: &Row, rect: Rect) {
    let theme = *ui.theme();
    let kind = if row.is_selected {
        TextKind::Strong
    } else {
        TextKind::Medium
    };
    let subject_style = ui.text_style(kind);
    let detail_style = ui.text_style(TextKind::Detail);
    let gap = theme.sizes.spacing.tiny + 1.0;
    let first = ui.line_height(subject_style);
    let second = ui.line_height(detail_style);
    let top = place(rect, (rect.width, first + gap + second), (Align::Start, Align::Center)).y;
    let references = data::references(row.index);
    let badges: Vec<Badge<'_>> = references
        .iter()
        .map(|reference| Badge {
            text: reference.name,
            icons: &[],
            tone: Tone::Lane(reference.lane),
            fill: BadgeFill::Tint,
        })
        .collect();
    let pills: f32 = badges
        .iter()
        .map(|badge| ui.badge_width(badge) + theme.sizes.spacing.medium)
        .sum();
    let subject = TextRun {
        text: data::subject(row.index),
        style: subject_style,
        color: row.foreground,
        origin: Point::new(rect.x, top),
        max_width: Some((rect.width - pills).max(0.0)),
    };
    let mut x = rect.x + ui.text(&subject) + theme.sizes.spacing.medium;
    for (badge, reference) in badges.iter().zip(references) {
        let width = ui.badge_width(badge);
        let cell = Rect::new(x, top, width, first);
        let drawn = ui.badge(cell, badge);
        let _ = ui.drag_source(("reference", reference.name), drawn, reference.name);
        x += width + theme.sizes.spacing.medium;
    }
    let author = data::author(row.index);
    let detail = format!("{} \u{b7} {}", author.name, data::date(row.index));
    let run = TextRun {
        text: &detail,
        style: detail_style,
        color: theme.colors.muted_foreground,
        origin: Point::new(rect.x, top + first + gap),
        max_width: Some(rect.width),
    };
    let _ = ui.text(&run);
}
