//! The branch filter (`filter.picker`): the button before Quick search that says what the timeline shows, pressed in
//! while it is narrowed, and the list it opens: the presets, the box, every reference with a tick, and **Filter the
//! list of branches too**.

use model::filter::{BranchesShown, Preset, ReferenceKind, label, label_parts, preset, rank_names, tick, ticked};
use model::refs::References;
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{
    AccessRole, Axis, Border, Button, ButtonKind, Choice, Description, Icon, InputBox, Key, Label, NamedKey, Popover,
    Rect, Sense, TextKind, Tone, Ui,
};

use super::{ListToo, list_too_view};
use crate::intent::{FilterIntent, Intent};
use crate::line::{is_narrow, rows_of};
use crate::text::filter as words;

/// The key of the list.
const PICKER_KEY: &str = "branch filter";
/// How wide the list is.
const WIDTH: f32 = 320.0;
/// How wide the name on the button is at most, before it is cut.
const NAME_MOST: f32 = 160.0;
/// How many rows of references the list shows with room enough.
const LISTED_ROWS: usize = 12;

/// The branch filter: what the timeline shows, the references to choose from, and the list while it is open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BranchPicker<'a> {
    /// The branches shown; none is everything.
    pub shown: Option<&'a BranchesShown>,
    /// The references of the repository, in the order of `refs.sort`.
    pub references: &'a References,
    /// The list, while it is open.
    pub open: Option<PickerOpen<'a>>,
    /// Whether the branch rail lists only the branches the timeline shows: `branches.followGraphFilter`.
    pub follows_timeline: bool,
}

/// The list of the branch filter while it is open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickerOpen<'a> {
    /// What is typed in its box.
    pub typed: &'a str,
    /// The size of the window it opened in, in logical pixels: a window resized closes it.
    pub window: (f32, f32),
}

/// A row of the list of references.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum PickRow<'a> {
    /// **Local**, **Remotes** or **Tags**.
    Heading(&'static str),
    /// A reference with its tick.
    Reference {
        /// Its full name.
        full: String,
        /// Its name as listed.
        name: &'a str,
        /// Whether it is the branch checked out.
        is_checked_out: bool,
    },
    /// What the list says with no reference in it.
    Empty(&'static str),
}

/// How wide the button is: the icon alone in the narrow layout, otherwise the icon, the name and the count.
#[must_use]
pub fn picker_button_width(state: &BranchPicker<'_>, ui: &Ui<'_>) -> f32 {
    let theme = ui.theme();
    let spacing = theme.sizes.spacing;
    if is_narrow(ui) {
        return theme.sizes.control_height;
    }
    let (name, count) = label_parts(state.shown);
    let style = ui.text_style(TextKind::Body);
    let name_width = ui.measure(&name, style).min(NAME_MOST);
    let count_width = count.map_or(0.0, |count| ui.measure(&format!("+{count}"), style) + spacing.small);
    2.0 * spacing.medium + theme.sizes.icon + spacing.small + name_width + count_width
}

/// The button of the branch filter over the rectangle its caller clipped it to, and its list while it is open.
///
/// Feature: filter.picker
pub fn picker_view(state: &BranchPicker<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let clicked = button(state, ui, rect);
    let window = ui.window();
    match (&state.open, clicked) {
        (None, true) => vec![Intent::Filter(FilterIntent::OpenPicker {
            window: (window.width, window.height),
        })],
        (Some(_), true) => vec![Intent::Filter(FilterIntent::ClosePicker)],
        (Some(open), false) => {
            let is_resized = (open.window.0 - window.width).abs() > 0.5 || (open.window.1 - window.height).abs() > 0.5;
            if is_resized {
                vec![Intent::Filter(FilterIntent::ClosePicker)]
            } else {
                list(state, open, ui, rect)
            }
        }
        (None, false) => Vec::new(),
    }
}

/// Draws the button, and says whether it was clicked.
fn button(state: &BranchPicker<'_>, ui: &mut Ui<'_>, rect: Rect) -> bool {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let id = ui.id("branch filter button");
    let response = ui.interact(id, rect, Sense::CLICK | Sense::FOCUS);
    let is_pressed = state.shown.is_some();
    let radius = theme.sizes.radius.large;
    if is_pressed {
        ui.fill_rounded_rect(rect, radius, theme.colors.list_inactive_selection);
        let width = theme.sizes.border;
        ui.border(
            rect,
            Border {
                width,
                radius,
                color: theme.colors.border,
            },
        );
    }
    if response.hovered || state.open.is_some() {
        ui.fill_rounded_rect(rect, radius, theme.colors.list_hover);
    }
    let tone = if is_pressed { Tone::Accent } else { Tone::Normal };
    let what = label(state.shown);
    if is_narrow(ui) {
        let cell = place(
            rect,
            (theme.sizes.icon, theme.sizes.icon),
            (Align::Center, Align::Center),
        );
        ui.icon_in(cell, Icon::Filter, tone);
    } else {
        let mut parts = Area::with_gap(inset(rect, Insets::symmetric(spacing.medium, 0.0)), spacing.small);
        ui.icon_in(parts.left(theme.sizes.icon), Icon::Filter, tone);
        let (name, count) = label_parts(state.shown);
        if let Some(count) = count {
            let count = format!("+{count}");
            let width = ui.measure(&count, ui.text_style(TextKind::Body));
            let _ = ui.label(parts.right(width), &Label::new(&count).tone(Tone::Muted));
        }
        let _ = ui.label(parts.rest(), &Label::new(&name));
    }
    if response.shows_focus {
        ui.focus_ring(rect, radius);
    }
    let tooltip = words::button_tooltip(&what);
    ui.tooltip(&response, &tooltip);
    let description = Description::new(AccessRole::Button, &tooltip)
        .on(is_pressed)
        .expanded(state.open.is_some());
    ui.describe(id, rect, &description);
    response.clicked
}

/// What the list is drawn from.
struct Body<'s, 'a> {
    /// The branch filter.
    state: &'s BranchPicker<'a>,
    /// The list open.
    open: &'s PickerOpen<'a>,
    /// Its rows of references.
    rows: Vec<PickRow<'a>>,
}

/// The list under the button, as tall as the room there is.
fn list(state: &BranchPicker<'_>, open: &PickerOpen<'_>, ui: &mut Ui<'_>, anchor: Rect) -> Vec<Intent> {
    let sizes = ui.theme().sizes;
    let rows = pick_rows(state.references, open.typed);
    let listed = rows.len().clamp(1, LISTED_ROWS);
    let height = 2.0 * sizes.control_height
        + sizes.row_height * (rows_of(listed) + 1.0)
        + 5.0 * sizes.spacing.medium
        + sizes.border;
    let popover = Popover {
        anchor,
        size: (WIDTH, height),
        name: words::PICKER,
    };
    let body = Body { state, open, rows };
    let response = ui.popover(PICKER_KEY, &popover, |ui, rect, is_new| {
        draw_body(&body, ui, rect, is_new)
    });
    let mut intents = response.inner;
    if response.is_dismissed {
        intents.push(Intent::Filter(FilterIntent::ClosePicker));
    }
    intents
}

/// A group of the list: its heading, the kind of its references, and their names, each with whether it is the branch
/// checked out.
type Group<'a> = (&'static str, ReferenceKind, Vec<(&'a str, bool)>);

/// The rows of references: under **Local**, **Remotes** and **Tags**, those whose names have what is typed, the best
/// first; with none, what the list says.
pub(super) fn pick_rows<'a>(references: &'a References, typed: &str) -> Vec<PickRow<'a>> {
    let groups: [Group<'a>; 3] = [
        (
            words::LOCAL,
            ReferenceKind::Local,
            references
                .branches
                .iter()
                .map(|branch| (branch.name.as_str(), branch.is_checked_out))
                .collect(),
        ),
        (
            words::REMOTES,
            ReferenceKind::Remote,
            references
                .remote_branches
                .iter()
                .map(|branch| (branch.name.as_str(), false))
                .collect(),
        ),
        (
            words::TAGS,
            ReferenceKind::Tag,
            references.tags.iter().map(|tag| (tag.name.as_str(), false)).collect(),
        ),
    ];
    let mut rows = Vec::new();
    let mut has_any = false;
    for (heading, kind, names) in groups {
        has_any |= !names.is_empty();
        let owned: Vec<String> = names.iter().map(|(name, _)| (*name).to_owned()).collect();
        let ranked = rank_names(&owned, typed);
        if ranked.is_empty() {
            continue;
        }
        rows.push(PickRow::Heading(heading));
        rows.extend(
            ranked
                .into_iter()
                .filter_map(|index| names.get(index))
                .map(|(name, is_checked_out)| PickRow::Reference {
                    full: kind.full_name(name),
                    name,
                    is_checked_out: *is_checked_out,
                }),
        );
    }
    if rows.is_empty() {
        rows.push(PickRow::Empty(if has_any {
            words::NO_NAME_MATCHES
        } else {
            words::NO_REFERENCES
        }));
    }
    rows
}

/// The inside of the list: the presets, the box, the references and **Filter the list of branches too**.
fn draw_body(body: &Body<'_, '_>, ui: &mut Ui<'_>, rect: Rect, is_new: bool) -> Vec<Intent> {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut intents = Vec::new();
    let mut area = Area::with_gap(inset(rect, Insets::all(spacing.medium)), spacing.medium);
    intents.extend(presets(body.state.shown, ui, area.top(theme.sizes.control_height)));
    let box_rect = area.top(theme.sizes.control_height);
    let input_id = ui.id("filter box");
    if is_new {
        ui.request_focus(input_id);
    }
    let input = InputBox::new(body.open.typed, words::BOX).with_icon(Icon::Filter);
    if let Some(text) = ui.input_box("filter box", box_rect, &input).changed_text {
        intents.push(Intent::Filter(FilterIntent::Type(text)));
    }
    let bottom = area.bottom(theme.sizes.row_height);
    let line = area.bottom(theme.sizes.border);
    ui.separator(line, Axis::Horizontal);
    let list_too = ListToo {
        is_on: body.state.follows_timeline,
    };
    intents.extend(ui.clip(bottom, |ui| list_too_view(&list_too, ui)));
    intents.extend(reference_rows(body, ui, area.rest()));
    let is_escaped = ui
        .unclaimed_keys()
        .iter()
        .any(|key| key.key == Key::Named(NamedKey::Escape));
    if is_escaped {
        intents.push(Intent::Filter(if body.open.typed.is_empty() {
            FilterIntent::ClosePicker
        } else {
            FilterIntent::Type(String::new())
        }));
    }
    intents
}

/// The presets, side by side, the one in use filled.
fn presets(shown: Option<&BranchesShown>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let spacing = ui.theme().sizes.spacing;
    let mut area = Area::with_gap(rect, spacing.small);
    let mut intents = Vec::new();
    for (choice, name, tooltip) in [
        (Preset::All, words::ALL, words::ALL_TOOLTIP),
        (Preset::Current, words::CURRENT, words::CURRENT_TOOLTIP),
        (Preset::Upstream, words::UPSTREAM, words::UPSTREAM_TOOLTIP),
        (Preset::Local, words::LOCAL, words::LOCAL_TOOLTIP),
        (Preset::Remotes, words::REMOTES, words::REMOTES_TOOLTIP),
    ] {
        let chosen = preset(choice);
        let kind = if chosen.as_ref() == shown {
            ButtonKind::Secondary
        } else {
            ButtonKind::Icon
        };
        let button = Button::secondary(name).kind(kind).with_tooltip(tooltip);
        let (width, _) = ui.button_size(&button);
        if ui.button(("preset", name), area.left(width), &button).clicked {
            intents.push(Intent::Filter(FilterIntent::Show(chosen)));
        }
    }
    intents
}

/// The references with their ticks, in a list that scrolls.
fn reference_rows(body: &Body<'_, '_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let theme = *ui.theme();
    let row_height = theme.sizes.row_height;
    let shown = body.state.shown;
    let references = body.state.references;
    let names = ticked(shown, references);
    let content = (rect.width, row_height * rows_of(body.rows.len()));
    let mut intents = Vec::new();
    let _ = ui.scroll_area("references", rect, content, |ui, inner| {
        for (index, row) in body.rows.iter().enumerate() {
            let row_rect = Rect::new(inner.x, inner.y + row_height * rows_of(index), inner.width, row_height);
            if row_rect.bottom() < rect.y || row_rect.y > rect.bottom() {
                continue;
            }
            intents.extend(pick_row(ui, row, row_rect, (shown, references, &names)));
        }
    });
    intents
}

/// Draws a row of the list, and says what its tick asks.
fn pick_row(
    ui: &mut Ui<'_>,
    row: &PickRow<'_>,
    rect: Rect,
    (shown, references, names): (Option<&BranchesShown>, &References, &[String]),
) -> Option<Intent> {
    let theme = *ui.theme();
    match row {
        PickRow::Heading(text) => {
            let _ = ui.label(rect, &Label::new(text).kind(TextKind::SmallStrong).tone(Tone::Muted));
            None
        }
        PickRow::Empty(text) => {
            let _ = ui.label(rect, &Label::new(text).tone(Tone::Muted));
            None
        }
        PickRow::Reference {
            full,
            name,
            is_checked_out,
        } => {
            if *is_checked_out {
                let dot = theme.sizes.spacing.small;
                let center = ui::Point::new(rect.right() - 2.0 * dot, rect.center().y);
                ui.fill_circle(center, dot * 0.5 + 1.0, theme.colors.accent);
            }
            let is_ticked = names.iter().any(|ticked| ticked == full);
            let clicked = ui
                .choice(("tick", full), rect, &Choice::check_box(name, is_ticked))
                .clicked;
            clicked.then(|| Intent::Filter(FilterIntent::Show(tick(shown, full, references))))
        }
    }
}
