//! The list a branch filter opens, for the timeline (`filter.picker`) or for the branch rail alone
//! (`refs.list-filter`): the presets, the box, every reference with a tick and, for the rail, the kinds of references
//! it lists.

use model::filter::{BranchesShown, Preset, ReferenceKind, kinds, kinds_shown, preset, rank_names, tick, ticked};
use model::refs::References;
use ui::layout::{Area, Insets, inset};
use ui::{Choice, InputBox, Key, Label, NamedKey, Popover, Rect, TextKind, Tone, Ui};

use crate::icons;
use crate::intent::{FilterIntent, Intent, RailIntent};
use crate::line::rows_of;
use crate::text::filter as words;
use crate::text::rail as rail_words;

/// How wide the list is.
const WIDTH: f32 = 320.0;
/// How many rows of references the list shows with room enough.
const LISTED_ROWS: usize = 12;

/// Which filter a list chooses for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PickerFor {
    /// The branches the timeline shows (`filter.picker`).
    Timeline,
    /// The branches the branch rail lists (`refs.list-filter`).
    Rail,
}

impl PickerFor {
    /// What choosing some branches asks; none is everything.
    fn choose(self, chosen: Option<BranchesShown>) -> Intent {
        match self {
            PickerFor::Timeline => Intent::Filter(FilterIntent::Show(chosen)),
            PickerFor::Rail => Intent::Rail(RailIntent::List(chosen)),
        }
    }

    /// What closing the list asks.
    pub(crate) fn close(self) -> Intent {
        match self {
            PickerFor::Timeline => Intent::Filter(FilterIntent::ClosePicker),
            PickerFor::Rail => Intent::Rail(RailIntent::CloseListFilter),
        }
    }

    /// What typing in the box asks.
    fn typed(self, text: String) -> Intent {
        match self {
            PickerFor::Timeline => Intent::Filter(FilterIntent::Type(text)),
            PickerFor::Rail => Intent::Rail(RailIntent::TypeInListFilter(text)),
        }
    }

    /// The key of the list, and its name for the screen reader.
    fn key_and_name(self) -> (&'static str, &'static str) {
        match self {
            PickerFor::Timeline => ("branch filter", words::PICKER),
            PickerFor::Rail => ("rail filter", rail_words::LIST_FILTER),
        }
    }
}

/// A list of a branch filter, open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Picking<'a> {
    /// The filter it chooses for.
    pub(crate) target: PickerFor,
    /// What the filter chose; none is everything.
    pub(crate) chosen: Option<&'a BranchesShown>,
    /// The references of the repository, in the order of `refs.sort`.
    pub(crate) references: &'a References,
    /// What is typed in its box.
    pub(crate) typed: &'a str,
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

/// What the list is drawn from.
struct Body<'s, 'a> {
    /// The list open.
    picking: &'s Picking<'a>,
    /// Its rows of references.
    rows: Vec<PickRow<'a>>,
}

/// The list under the control it opened from, as tall as the room there is.
pub(crate) fn picker_list(picking: &Picking<'_>, ui: &mut Ui<'_>, anchor: Rect) -> Vec<Intent> {
    let sizes = ui.theme().sizes;
    let rows = pick_rows(picking.references, picking.typed);
    let listed = rows.len().clamp(1, LISTED_ROWS);
    let mut height = 2.0 * sizes.control_height + sizes.row_height * rows_of(listed) + 4.0 * sizes.spacing.medium;
    if picking.target == PickerFor::Rail {
        height += sizes.row_height * rows_of(ReferenceKind::ALL.len()) + sizes.spacing.medium;
    }
    let (key, name) = picking.target.key_and_name();
    let popover = Popover {
        anchor,
        size: (WIDTH, height),
        name,
    };
    let body = Body { picking, rows };
    let response = ui.popover(key, &popover, |ui, rect, is_new| draw_body(&body, ui, rect, is_new));
    let mut intents = response.inner;
    if response.is_dismissed {
        intents.push(picking.target.close());
    }
    intents
}

/// A group of the list: its heading, the kind of its references, and their names, each with whether it is the branch
/// checked out.
type Group<'a> = (&'static str, ReferenceKind, Vec<(&'a str, bool)>);

/// The rows of references: under **Local**, **Remotes** and **Tags** while nothing is typed; while something is, one list
/// of those whose names have it, the best first, and among those that match as well in the order of the groups; with
/// none, what the list says.
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
    let has_any = groups.iter().any(|(_, _, names)| !names.is_empty());
    let mut rows = if typed.is_empty() {
        grouped_rows(groups)
    } else {
        matched_rows(groups, typed)
    };
    if rows.is_empty() {
        rows.push(PickRow::Empty(if has_any {
            words::NO_NAME_MATCHES
        } else {
            words::NO_REFERENCES
        }));
    }
    rows
}

/// The references under the heading of their group, in their order.
fn grouped_rows(groups: [Group<'_>; 3]) -> Vec<PickRow<'_>> {
    let mut rows = Vec::new();
    for (heading, kind, names) in groups {
        if names.is_empty() {
            continue;
        }
        rows.push(PickRow::Heading(heading));
        rows.extend(names.into_iter().map(|(name, is_checked_out)| PickRow::Reference {
            full: kind.full_name(name),
            name,
            is_checked_out,
        }));
    }
    rows
}

/// The references whose names have what is typed, in one list, the best first (`filter.rank-names`).
fn matched_rows<'a>(groups: [Group<'a>; 3], typed: &str) -> Vec<PickRow<'a>> {
    let all: Vec<(ReferenceKind, &'a str, bool)> = groups
        .into_iter()
        .flat_map(|(_, kind, names)| {
            names
                .into_iter()
                .map(move |(name, is_checked_out)| (kind, name, is_checked_out))
        })
        .collect();
    let owned: Vec<String> = all.iter().map(|(_, name, _)| (*name).to_owned()).collect();
    rank_names(&owned, typed)
        .into_iter()
        .filter_map(|index| all.get(index))
        .map(|(kind, name, is_checked_out)| PickRow::Reference {
            full: kind.full_name(name),
            name,
            is_checked_out: *is_checked_out,
        })
        .collect()
}

/// The inside of the list: the presets, the box, the references and, for the rail, the kinds it lists.
fn draw_body(body: &Body<'_, '_>, ui: &mut Ui<'_>, rect: Rect, is_new: bool) -> Vec<Intent> {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let picking = body.picking;
    let mut intents = Vec::new();
    let mut area = Area::with_gap(inset(rect, Insets::all(spacing.medium)), spacing.medium);
    intents.extend(presets(picking, ui, area.top(theme.sizes.control_height)));
    let box_rect = area.top(theme.sizes.control_height);
    let input_id = ui.id("filter box");
    if is_new {
        ui.request_focus(input_id);
    }
    let input = InputBox::new(picking.typed, words::BOX).with_icon(icons::FILTER);
    if let Some(text) = ui.input_box("filter box", box_rect, &input).changed_text {
        intents.push(picking.target.typed(text));
    }
    if picking.target == PickerFor::Rail {
        let kinds_rect = area.bottom(theme.sizes.row_height * rows_of(ReferenceKind::ALL.len()));
        intents.extend(kind_rows(picking.chosen, ui, kinds_rect));
    }
    intents.extend(reference_rows(body, ui, area.rest()));
    let is_escaped = ui
        .unclaimed_keys()
        .iter()
        .any(|key| key.key == Key::Named(NamedKey::Escape));
    if is_escaped {
        intents.push(if picking.typed.is_empty() {
            picking.target.close()
        } else {
            picking.target.typed(String::new())
        });
    }
    intents
}

/// The presets, side by side, the one in use filled.
fn presets(picking: &Picking<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
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
        let kind = if chosen.as_ref() == picking.chosen {
            ui::ButtonKind::Secondary
        } else {
            ui::ButtonKind::Icon
        };
        let button = ui::Button::secondary(name).kind(kind).with_tooltip(tooltip);
        let (width, _) = ui.button_size(&button);
        if ui.button(("preset", name), area.left(width), &button).clicked {
            intents.push(picking.target.choose(chosen));
        }
    }
    intents
}

/// **Show local branches**, **Show remote branches** and **Show tags**, the choice of the rail's filter in kinds
/// (`filter.kinds`): greyed out while references are ticked one by one, or **Current** or **Upstream** is chosen, and
/// the one kind left on greyed out too.
fn kind_rows(chosen: Option<&BranchesShown>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let row_height = ui.theme().sizes.row_height;
    let shown = kinds_shown(chosen);
    let mut intents = Vec::new();
    for (index, (kind, name)) in [
        (ReferenceKind::Local, words::SHOW_LOCAL),
        (ReferenceKind::Remote, words::SHOW_REMOTE),
        (ReferenceKind::Tag, words::SHOW_TAGS),
    ]
    .into_iter()
    .enumerate()
    {
        let row = Rect::new(rect.x, rect.y + row_height * rows_of(index), rect.width, row_height);
        let is_on = shown.is_some_and(|kinds| kinds.has(kind));
        let is_last_on = shown.is_some_and(|kinds| kinds.has(kind) && kinds.count() == 1);
        let choice = Choice::check_box(name, is_on);
        let choice = if shown.is_none() || is_last_on {
            choice.disabled()
        } else {
            choice
        };
        let response = ui.choice(("kind", name), row, &choice);
        if shown.is_none() {
            ui.tooltip(&response, words::CHOSEN_IN_FILTER);
        }
        if response.clicked {
            intents.push(Intent::Rail(RailIntent::List(kinds(chosen, kind))));
        }
    }
    intents
}

/// The references with their ticks, in a list that scrolls.
fn reference_rows(body: &Body<'_, '_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let theme = *ui.theme();
    let row_height = theme.sizes.row_height;
    let picking = body.picking;
    let names = ticked(picking.chosen, picking.references);
    let content = (rect.width, row_height * rows_of(body.rows.len()));
    let mut intents = Vec::new();
    let _ = ui.scroll_area("references", rect, content, |ui, inner| {
        for (index, row) in body.rows.iter().enumerate() {
            let row_rect = Rect::new(inner.x, inner.y + row_height * rows_of(index), inner.width, row_height);
            if row_rect.bottom() < rect.y || row_rect.y > rect.bottom() {
                continue;
            }
            intents.extend(pick_row(ui, row, row_rect, (picking, &names)));
        }
    });
    intents
}

/// Draws a row of the list, and says what its tick asks.
fn pick_row(
    ui: &mut Ui<'_>,
    row: &PickRow<'_>,
    rect: Rect,
    (picking, names): (&Picking<'_>, &[String]),
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
            clicked.then(|| picking.target.choose(tick(picking.chosen, full, picking.references)))
        }
    }
}
