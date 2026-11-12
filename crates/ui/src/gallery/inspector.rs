//! The inspector of the commit picked in the timeline, in a card on the right: its hash and date, its message, who
//! wrote it, the files it changed with the lines added and removed, and what can be done with it. In a narrow window
//! it opens over the timeline instead, with a button that closes it.

use render::{Icon, Point, Rect, TextRun};

use super::menus;
use super::{GalleryIntent, data};
use crate::access::AccessRole;
use crate::frame::{TextKind, Ui};
use crate::layer::Layer;
use crate::layout::{Align, Area, Insets, inset, place};
use crate::widget::Tone;
use crate::widget::button::{Button, ButtonKind};
use crate::widget::find::ListSearch;
use crate::widget::label::Label;
use crate::widget::list::{List, Row, SelectionMode};
use crate::widget::metrics;

/// The height of the rows of the files.
const FILE_ROW: f32 = 30.0;
/// The side of the round picture of the author.
const AVATAR: f32 = 28.0;
/// The most lines of the message the card shows.
const BODY_LINES: usize = 3;

/// Declares the inspector in its column beside the timeline.
pub(super) fn inspector(ui: &mut Ui<'_>, column: Rect, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let padding = spacing.large + spacing.small;
    let card = inset(
        column,
        Insets {
            left: 0.0,
            top: padding,
            right: padding,
            bottom: padding,
        },
    );
    let radius = card_radius(ui);
    ui.card(card, radius);
    ui.region(
        "inspector region",
        (card, AccessRole::Complementary, "Selected commit"),
        |ui| {
            contents(ui, card, intents);
        },
    );
}

/// Declares the inspector over the right of the timeline, in a narrow window, with a button that closes it.
pub(super) fn inspector_over(ui: &mut Ui<'_>, main: Rect, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    let padding = theme.sizes.spacing.large + theme.sizes.spacing.small;
    let width = theme.sizes.inspector_width.min(main.width - 2.0 * padding).max(0.0);
    let card = Rect::new(
        main.right() - padding - width,
        main.y + padding,
        width,
        (main.height - 2.0 * padding).max(0.0),
    );
    ui.layer(Layer::Popup, |ui| {
        let radius = card_radius(ui);
        ui.floating_card(card, radius);
        let blocker = ui.id("inspector over");
        let _ = ui.interact(blocker, card, crate::interaction::Sense::CLICK);
        ui.region(
            "inspector region",
            (card, AccessRole::Complementary, "Selected commit"),
            |ui| {
                let side = metrics::control_height(ui.theme());
                let close = Rect::new(card.right() - padding - side, card.y + padding * 0.5, side, side);
                let button = Button::icon(Icon::Close, "Close the commit").kind(ButtonKind::Icon);
                if ui.button("close inspector", close, &button).clicked {
                    intents.push(GalleryIntent::CloseInspector);
                }
                contents(ui, card, intents);
            },
        );
    });
}

/// The radius of the card of the inspector, a little rounder than the others.
fn card_radius(ui: &Ui<'_>) -> f32 {
    let radius = ui.theme().sizes.radius;
    radius.large + radius.medium
}

/// What the card holds, for the commit picked.
fn contents(ui: &mut Ui<'_>, card: Rect, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let inner = inset(card, Insets::all(spacing.huge));
    let Some(row) = super::picked_commit(ui) else {
        let words = Label::new("Pick a commit to see it here.").tone(Tone::Muted);
        let _ = ui.label(Rect::new(inner.x, inner.y, inner.width, theme.sizes.row_height), &words);
        return;
    };
    let mut area = Area::with_gap(inner, spacing.large + spacing.small);
    message(ui, &mut area, row);
    author(ui, area.top(AVATAR + spacing.tiny), row);
    let height = metrics::control_height(&theme);
    let buttons = area.bottom(height);
    files(ui, &mut area, row);
    actions(ui, buttons, row, intents);
}

/// The hash and the date, the subject as a title, and the rest of the message.
fn message(ui: &mut Ui<'_>, area: &mut Area, row: usize) {
    let theme = *ui.theme();
    let hash_style = ui.text_style(TextKind::Hash);
    let heading = ui.text_style(TextKind::Heading);
    let body = ui.text_style(TextKind::Body);
    let gap = theme.sizes.spacing.medium;
    let text = data::body(row);
    let lines = ui.fonts().wrap(text, body, area_width(area)).len().min(BODY_LINES);
    let body_height = crate::number::float_of_count(lines) * ui.line_height(body);
    let height = ui.line_height(hash_style) + gap + ui.line_height(heading) + gap + body_height;
    let mut parts = Area::with_gap(area.top(height), gap);
    let when = format!("{} \u{b7} {}", data::short_hash(row), data::date(row));
    let first = parts.top(ui.line_height(hash_style));
    let _ = ui.label(first, &Label::new(&when).kind(TextKind::Hash).tone(Tone::Muted));
    let _ = ui.heading(parts.top(ui.line_height(heading)), data::subject(row));
    let _ = ui.paragraph(parts.rest(), text, (TextKind::Body, Tone::Muted));
}

/// The width of what is left of an area.
fn area_width(area: &Area) -> f32 {
    (*area).rest().width
}

/// The round picture of the author, with the name and the e-mail address.
fn author(ui: &mut Ui<'_>, rect: Rect, row: usize) {
    let theme = *ui.theme();
    let person = data::author(row);
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.medium + 2.0);
    let avatar = place(parts.left(AVATAR), (AVATAR, AVATAR), (Align::Start, Align::Center));
    ui.fill_circle(avatar.center(), AVATAR * 0.5, theme.colors.secondary_button_background);
    let initials = Label::new(person.initials)
        .kind(TextKind::SmallStrong)
        .align(Align::Center);
    let _ = ui.draw_label(avatar, &initials);
    let mut lines = Area::new(parts.rest());
    let half = rect.height * 0.5;
    let _ = ui.label(lines.top(half), &Label::new(person.name).kind(TextKind::Medium));
    let email = Label::new(person.email).kind(TextKind::Detail).tone(Tone::Muted);
    let _ = ui.label(lines.rest(), &email);
}

/// How many files the commit changed with how many lines in all, over a list of the files.
fn files(ui: &mut Ui<'_>, area: &mut Area, row: usize) {
    let theme = *ui.theme();
    let changed = data::files(row);
    let added: u32 = changed.iter().map(|file| file.added).sum();
    let removed: u32 = changed.iter().map(|file| file.removed).sum();
    let mut header = Area::new(area.top(theme.sizes.row_height));
    let count = format!("{} files", changed.len());
    let lines = format!("+{added} \u{2212}{removed}");
    let detail = |text| Label::new(text).kind(TextKind::Detail).tone(Tone::Muted);
    let lines_width = ui.measure(&lines, ui.text_style(TextKind::Detail));
    let _ = ui.label(header.right(lines_width), &detail(&lines));
    let _ = ui.label(header.rest(), &detail(&count));
    let shown = FILE_ROW * crate::number::float_of_count(changed.len());
    let rect = area.top(shown.min(area_height(area)));
    let list = List {
        name: "Files",
        row_count: changed.len(),
        row_height: FILE_ROW,
        selection_mode: SelectionMode::Single,
        lead: 0.0,
    };
    let file_text = |index: usize| {
        changed
            .get(index)
            .map(|file| format!("{} +{} \u{2212}{}", file.path, file.added, file.removed))
            .unwrap_or_default()
    };
    let search = ListSearch {
        row_text: &file_text,
        generation: crate::number::wide_of_count(row).to_bits(),
    };
    let _ = ui.searchable_list("files", (rect, &list), &search, |ui, file| file_row(ui, file, row));
}

/// The height of what is left of an area.
fn area_height(area: &Area) -> f32 {
    (*area).rest().height
}

/// A row of the files: the path, then the lines added and removed in their colors.
fn file_row(ui: &mut Ui<'_>, file: &Row, commit: usize) {
    let Some(changed) = data::files(commit).get(file.index) else {
        return;
    };
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let style = ui.text_style(TextKind::Detail);
    let mut parts = Area::with_gap(inset(file.rect, Insets::symmetric(spacing.medium, 0.0)), spacing.medium);
    let removed = format!("\u{2212}{}", changed.removed);
    let added = format!("+{}", changed.added);
    for (text, color) in [(&removed, theme.colors.removed), (&added, theme.colors.added)] {
        let width = ui.measure(text, style);
        let cell = parts.right(width);
        let line = place(cell, (width, ui.line_height(style)), (Align::Start, Align::Center));
        let run = TextRun {
            text,
            style,
            color,
            origin: Point::new(line.x, line.y),
            max_width: None,
        };
        let _ = ui.text(&run);
    }
    let _ = ui.draw_label(parts.rest(), &Label::new(changed.path));
}

/// Show the changes, Cherry-pick, and the menu of the rest.
fn actions(ui: &mut Ui<'_>, rect: Rect, row: usize, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    let mut area = Area::with_gap(rect, theme.sizes.spacing.medium);
    let more = Button::icon(Icon::More, "More actions").kind(ButtonKind::Outline);
    let resets = menus::reset_menu();
    let entries = menus::row_menu(&resets);
    let side = rect.height;
    if let Some(command) = ui.menu_button("commit menu", area.right(side), &more, &entries) {
        menus::run_command(ui, command, Some(row), intents);
    }
    let pick = Button::outline("Cherry-pick");
    let (width, _) = ui.button_size(&pick);
    if ui.button("cherry-pick", area.right(width), &pick).clicked {
        menus::run_command(ui, menus::MenuCommand::CherryPick, Some(row), intents);
    }
    let show = Button::outline("Show the changes");
    if ui.button("show the changes", area.rest(), &show).clicked {
        intents.push(GalleryIntent::Clicked(String::from("Show the changes")));
    }
}
