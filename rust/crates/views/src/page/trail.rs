//! The title row of a page: the button of the side column, the trail of the pages it was reached through, the name of
//! the page and the button that closes it (`ui.page-trail`, `repo.side-column`).

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Axis, Button, ButtonKind, Key, Label, MenuEntry, Modifiers, NamedKey, Rect, TextKind, Tone, Ui};

use super::PageHostState;
use crate::graph::{ReferencesShown, side_button};
use crate::icons;
use crate::intent::Intent;
use crate::text::pages as words;

/// The widest a name before the page's own is drawn, in logical pixels; a longer one is cut at its end with `…`.
const NAME_MOST: f32 = 200.0;

/// A name of the trail before the page's own: its words, and what a click on it asks.
struct Name<'a> {
    /// What it says.
    words: &'a str,
    /// What a click on it asks.
    intent: Intent,
}

/// The names of the trail before the page's own: **Timeline**, the pages the page was reached through, each going back
/// to it, and the step of the page inside it when it has one.
fn names_before<'a>(host: &'a PageHostState<'a>) -> Vec<Name<'a>> {
    let timeline = Name {
        words: words::TIMELINE,
        intent: Intent::ShowTimeline,
    };
    let way = host.way.iter().map(|(page, name)| Name {
        words: name,
        intent: Intent::BackTo(*page),
    });
    let step = host.page.step().map(|(words, intent)| Name { words, intent });
    std::iter::once(timeline).chain(way).chain(step).collect()
}

/// What `Alt+Left` asks: the name before the page's own, **Timeline** when it is the only one.
pub(super) fn back_one(host: &PageHostState<'_>) -> Intent {
    names_before(host)
        .pop()
        .map_or(Intent::ShowTimeline, |name| name.intent)
}

/// Whether `Alt+Left` was pressed and nothing used it.
pub(super) fn is_back_pressed(ui: &Ui<'_>) -> bool {
    ui.unclaimed_keys()
        .iter()
        .any(|key| key.key == Key::Named(NamedKey::ArrowLeft) && key.modifiers == Modifiers::ALT)
}

/// The title row above a page: the button of the side column when there is one, at the same place as in the top row
/// of the timeline (`repo.side-column`), the trail, a dot after the name of a file with edits not saved, and the button
/// that closes the page. At most four names are shown, **Timeline** and the page's own among them; those in between
/// are folded into **…**, which lists them (`ui.page-trail`).
pub(super) fn trail_view(
    host: &PageHostState<'_>,
    side: Option<ReferencesShown>,
    ui: &mut Ui<'_>,
    rect: Rect,
) -> Vec<Intent> {
    let mut intents = Vec::new();
    let mut parts = row_start(side, ui, rect, &mut intents);
    ui.region("trail", (rect, AccessRole::Navigation, words::TRAIL), |ui| {
        let before = names_before(host);
        let folded = model::trail::folded(before.len() + 1).unwrap_or_default();
        for (index, name) in before.iter().enumerate() {
            if index == folded.start && !folded.is_empty() {
                intents.extend(folded_names(
                    before.get(folded.clone()).unwrap_or_default(),
                    ui,
                    &mut parts,
                ));
            } else if folded.contains(&index) {
                continue;
            } else {
                intents.extend(name_button(name, index, ui, &mut parts));
            }
            separator(ui, &mut parts);
        }
        let side = ui.theme().sizes.control_height;
        let close_cell = place(parts.right(side), (side, side), (Align::Start, Align::Center));
        let close = Button::icon(icons::CLOSE, words::CLOSE_PAGE).kind(ButtonKind::Icon);
        if ui.button("close page", close_cell, &close).clicked {
            intents.push(Intent::ClosePage);
        }
        title(host, ui, parts);
    });
    intents
}

/// A name before the page's own, as a button that asks what it goes back to.
fn name_button(name: &Name<'_>, index: usize, ui: &mut Ui<'_>, parts: &mut Area) -> Option<Intent> {
    let words = cut_at_end(ui, name.words, NAME_MOST);
    let button = Button::secondary(&words).kind(ButtonKind::Icon);
    let (width, height) = ui.button_size(&button);
    let cell = place(parts.left(width), (width, height), (Align::Start, Align::Center));
    ui.button(("trail name", index), cell, &button)
        .clicked
        .then(|| name.intent.clone())
}

/// **…** in place of the names folded, with its menu that lists them, the first opened first.
fn folded_names(names: &[Name<'_>], ui: &mut Ui<'_>, parts: &mut Area) -> Option<Intent> {
    let button = Button::secondary(words::FOLDED).kind(ButtonKind::Icon);
    let (width, height) = ui.button_size(&button);
    let cell = place(parts.left(width), (width, height), (Align::Start, Align::Center));
    let entries: Vec<MenuEntry<'_, usize>> = names
        .iter()
        .enumerate()
        .map(|(index, name)| MenuEntry::Item(icons::without_icon(name.words, index)))
        .collect();
    let chosen = ui.menu_button("trail folded", cell, &button, &entries)?;
    names.get(chosen).map(|name| name.intent.clone())
}

/// The name of the page shown, after the names before it, with a dot when it has edits not saved.
fn title(host: &PageHostState<'_>, ui: &mut Ui<'_>, mut parts: Area) {
    let title = host.page.title();
    if host.page.is_unsaved() {
        let dot = crate::text::file_view::UNSAVED_DOT;
        let title_width = ui.measure(title, ui.text_style(TextKind::Strong));
        let _ = ui.label(parts.left(title_width), &Label::new(title).kind(TextKind::Strong));
        let dot_width = ui.measure(dot, ui.text_style(TextKind::Body));
        let _ = ui.label(parts.left(dot_width), &Label::new(dot));
    } else {
        let _ = ui.label(parts.rest(), &Label::new(title).kind(TextKind::Strong));
    }
}

/// A name cut at its end with `…` to the room given, whole when it fits.
fn cut_at_end(ui: &Ui<'_>, name: &str, room: f32) -> String {
    let style = ui.text_style(TextKind::Body);
    if ui.measure(name, style) <= room {
        return name.to_owned();
    }
    for (end, _) in name.char_indices().rev() {
        let cut = format!("{}\u{2026}", name.get(..end).unwrap_or_default().trim_end());
        if ui.measure(&cut, style) <= room {
            return cut;
        }
    }
    "\u{2026}".to_owned()
}

/// The line under the title row, and its parts after the button of the side column when there is one, which is at the
/// left margin of the top row of the timeline; without it, the trail starts further in, as it always did.
fn row_start(side: Option<ReferencesShown>, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<Intent>) -> Area {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let line = Rect::new(
        rect.x,
        rect.bottom() - theme.sizes.border,
        rect.width,
        theme.sizes.border,
    );
    ui.separator(line, Axis::Horizontal);
    let margin = spacing.large + spacing.small;
    let left = if side.is_some() { spacing.medium } else { margin };
    let inner = inset(
        rect,
        Insets {
            left,
            right: margin,
            ..Insets::all(0.0)
        },
    );
    let mut parts = Area::with_gap(inner, spacing.medium);
    if let Some(side) = side {
        intents.extend(side_button(side, &mut parts, ui));
    }
    parts
}

/// The `›` between two names of the trail.
fn separator(ui: &mut Ui<'_>, parts: &mut Area) {
    let width = ui.measure(words::TRAIL_SEPARATOR, ui.text_style(TextKind::Body));
    let _ = ui.label(parts.left(width), &Label::new(words::TRAIL_SEPARATOR).tone(Tone::Muted));
}
