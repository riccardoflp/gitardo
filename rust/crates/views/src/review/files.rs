//! The files in conflict beside the cards (`review.file-list`), with where each one stands, and the filters.

use ui::layout::{Align, Area, place};
use ui::{AccessRole, Button, ButtonKind, Label, Rect, TextKind, Tone, Ui};

use super::{FileCards, Filter, ReviewIntent, ReviewState};
use crate::icons;
use crate::intent::Intent;
use crate::text::review as words;

/// The filters, with their names and what the pointer on them says.
const FILTERS: [(Filter, &str, Option<&str>); 4] = [
    (Filter::All, words::ALL, Some(words::ALL_TOOLTIP)),
    (Filter::NotAccepted, words::NOT_ACCEPTED, None),
    (Filter::NothingProposed, words::NOTHING_PROPOSED, None),
    (Filter::FromTheAi, words::FROM_THE_AI, None),
];

/// What the list shows, one under the other.
enum Entry<'a> {
    /// A filter.
    Filter(Filter, &'static str, Option<&'static str>),
    /// **All files**.
    AllFiles,
    /// A file with conflicts marked in it.
    File(&'a FileCards),
    /// The heading of the files deleted on one side.
    DeletedHeading,
    /// A file deleted on one side.
    Deleted(&'a str),
    /// The heading of the other files in conflict, and what is said under it.
    OthersHeading,
    /// The note under it.
    OthersNote,
    /// Another file in conflict.
    Other(&'a str),
    /// Nothing to show.
    Nothing,
}

/// The entries of the list.
fn entries<'a>(state: &ReviewState<'a>) -> Vec<Entry<'a>> {
    let mut entries: Vec<Entry<'a>> = FILTERS
        .iter()
        .map(|(filter, said, tooltip)| Entry::Filter(*filter, said, *tooltip))
        .collect();
    entries.push(Entry::AllFiles);
    let files: Vec<&FileCards> = state
        .files
        .iter()
        .filter(|file| {
            file.conflicts
                .iter()
                .any(|card| card.is_kept_by(state.filter) || state.kept_in_view.contains(&card.key))
        })
        .collect();
    let deleted: Vec<&str> = state
        .deleted
        .iter()
        .filter(|_| {
            matches!(
                state.filter,
                Filter::All | Filter::NotAccepted | Filter::NothingProposed
            )
        })
        .map(|file| file.path.as_str())
        .collect();
    if files.is_empty() && deleted.is_empty() {
        entries.push(Entry::Nothing);
    }
    entries.extend(files.into_iter().map(Entry::File));
    if !deleted.is_empty() {
        entries.push(Entry::DeletedHeading);
        entries.extend(deleted.into_iter().map(Entry::Deleted));
    }
    if !state.others.is_empty() {
        entries.push(Entry::OthersHeading);
        entries.push(Entry::OthersNote);
        entries.extend(state.others.iter().map(|path| Entry::Other(path)));
    }
    entries
}

/// The height of the list.
pub(super) fn list_height(state: &ReviewState<'_>, ui: &Ui<'_>) -> f32 {
    let sizes = ui.theme().sizes;
    entries(state)
        .iter()
        .map(|entry| match entry {
            Entry::Filter(..) | Entry::AllFiles | Entry::File(_) => sizes.control_height,
            Entry::OthersNote => sizes.row_height * 3.0,
            _ => sizes.row_height,
        })
        .sum()
}

/// The files in conflict, over the rectangle its caller clipped it to: the four filters with how many conflicts each
/// keeps, **All files**, each file with how many of its conflicts are accepted and its mark, the files deleted on one
/// side, and the other files in conflict.
///
/// Feature: review.file-list
pub fn file_list_view(state: &ReviewState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let sizes = ui.theme().sizes;
    let mut area = Area::new(rect);
    let mut intents = Vec::new();
    ui.region("review files", (rect, AccessRole::Navigation, words::FILE_LIST), |ui| {
        for entry in entries(state) {
            match entry {
                Entry::Filter(filter, said, tooltip) => {
                    let label = words::filter(said, state.count(filter));
                    let button = if state.filter == filter {
                        Button::secondary(&label)
                    } else {
                        Button::secondary(&label).kind(ButtonKind::Outline)
                    };
                    let button = tooltip.map_or(button, |tooltip| button.with_tooltip(tooltip));
                    if wide_button(ui, (area.top(sizes.control_height), ("filter", said)), &button) {
                        intents.push(Intent::Review(ReviewIntent::Filter(filter)));
                    }
                }
                Entry::AllFiles => {
                    let button = Button::secondary(words::ALL_FILES).kind(ButtonKind::Outline);
                    if wide_button(ui, (area.top(sizes.control_height), ("all files", "")), &button) {
                        intents.push(Intent::Review(ReviewIntent::ShowFile(None)));
                    }
                }
                Entry::File(file) => intents.extend(file_entry(state, ui, (file, area.top(sizes.control_height)))),
                Entry::DeletedHeading => heading(ui, area.top(sizes.row_height), words::DELETED_ON_ONE_SIDE),
                Entry::OthersHeading => heading(ui, area.top(sizes.row_height), words::ALSO_IN_CONFLICT),
                Entry::OthersNote => {
                    let _ = ui.paragraph(
                        area.top(sizes.row_height * 3.0),
                        words::ALSO_NOTE,
                        (TextKind::Small, Tone::Muted),
                    );
                }
                Entry::Deleted(path) => {
                    let button = Button::secondary(path).kind(ButtonKind::Outline);
                    if wide_button(ui, (area.top(sizes.row_height), ("deleted", path)), &button) {
                        intents.push(Intent::Review(ReviewIntent::ShowFile(Some(path.to_owned()))));
                    }
                }
                Entry::Other(path) => {
                    let button = Button::secondary(path).kind(ButtonKind::Icon);
                    if wide_button(ui, (area.top(sizes.row_height), ("other", path)), &button) {
                        let open = crate::conflicts::ConflictsIntent::Open(path.to_owned());
                        intents.push(Intent::Conflicts(open));
                    }
                }
                Entry::Nothing => {
                    let _ = ui.label(
                        area.top(sizes.row_height),
                        &Label::new(words::NOTHING_TO_SHOW).tone(Tone::Muted),
                    );
                }
            }
        }
    });
    intents
}

/// A button as wide as the list; whether it was clicked.
fn wide_button(ui: &mut Ui<'_>, (rect, key): (Rect, (&str, &str)), button: &Button<'_>) -> bool {
    let height = ui.button_size(button).1;
    let cell = place(rect, (rect.width, height), (Align::Start, Align::Center));
    ui.button(key, cell, button).clicked
}

/// The heading of a group of files.
fn heading(ui: &mut Ui<'_>, rect: Rect, said: &str) {
    let _ = ui.label(rect, &Label::new(said).kind(TextKind::SmallStrong).tone(Tone::Muted));
}

/// A file: its path, which shows its conflicts alone, how many of them are accepted, and a tick when all are or a
/// yellow dot when some have nothing proposed.
fn file_entry(state: &ReviewState<'_>, ui: &mut Ui<'_>, (file, rect): (&FileCards, Rect)) -> Vec<Intent> {
    let theme = *ui.theme();
    let mut area = Area::with_gap(rect, theme.sizes.spacing.small);
    let accepted = file.accepted();
    let total = file.conflicts.len();
    let side = theme.sizes.row_height;
    let mark = area.right(side);
    if accepted == total {
        ui.icon_in(mark, icons::CHOSEN, Tone::Success);
        let id = ui.id(("accepted", &file.path));
        let response = ui.interact(id, mark, ui::Sense::HOVER);
        ui.tooltip(&response, words::ALL_ACCEPTED);
    } else if file.has_unproposed() {
        let center = ui::Point::new(mark.x + mark.width * 0.5, mark.y + mark.height * 0.5);
        ui.fill_circle(center, theme.sizes.spacing.small, theme.colors.warning);
        let id = ui.id(("unproposed", &file.path));
        let response = ui.interact(id, mark, ui::Sense::HOVER);
        ui.tooltip(&response, words::SOME_UNPROPOSED);
    }
    let counted = words::accepted_of(accepted, total);
    let width = ui.measure(&counted, ui.text_style(TextKind::Small));
    let _ = ui.label(
        area.right(width),
        &Label::new(&counted).kind(TextKind::Small).tone(Tone::Muted),
    );
    let is_shown = state.file_shown == Some(file.path.as_str());
    let button = if is_shown {
        Button::secondary(&file.path)
    } else {
        Button::secondary(&file.path).kind(ButtonKind::Outline)
    };
    if wide_button(ui, (area.rest(), ("file", &file.path)), &button) {
        return vec![Intent::Review(ReviewIntent::ShowFile(Some(file.path.clone())))];
    }
    Vec::new()
}
