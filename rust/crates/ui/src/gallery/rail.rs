//! The branch rail, on the left of the window: your branches, each with the dot of its lane and how it stands, the
//! remote branches of origin, and what is kept aside — stashes, worktrees and tags — in sections that unfold. In a
//! narrow window it shows only the dots and icons.

use render::{Icon, LineCap, Rect, Stroke};

use super::data::{BRANCHES, KEPT, REMOTES};
use super::menus::{self, MenuCommand};
use super::{GalleryIntent, GalleryState, RailMode, widget_id};
use crate::access::{AccessRole, Description};
use crate::frame::{TextKind, Ui};
use crate::interaction::Sense;
use crate::layout::{Align, Area, Insets, inset, place};
use crate::widget::Tone;
use crate::widget::button::{Button, ButtonKind};
use crate::widget::find::ListSearch;
use crate::widget::label::Label;
use crate::widget::list::{List, Row, Selection, SelectionMode};
use crate::widget::panel::Axis;

/// The height of the rows of your branches.
const BRANCH_ROW: f32 = 32.0;
/// The height of the rows of the remote branches and of what is kept aside.
const QUIET_ROW: f32 = 30.0;
/// The height of the title of a section.
const TITLE: f32 = 20.0;
/// The side of the dot of a branch.
const DOT: f32 = 10.0;
/// The most entries an unfolded section shows.
const SHOWN_ENTRIES: usize = 4;

/// Declares the branch rail over its column, with the names or with icons only.
pub(super) fn rail(
    state: &GalleryState,
    ui: &mut Ui<'_>,
    (rect, mode): (Rect, RailMode),
    intents: &mut Vec<GalleryIntent>,
) {
    let theme = *ui.theme();
    ui.fill_rect(rect, theme.colors.rail_background);
    let edge = Rect::new(
        rect.right() - theme.sizes.border,
        rect.y,
        theme.sizes.border,
        rect.height,
    );
    ui.separator(edge, Axis::Vertical);
    ui.region("rail region", (rect, AccessRole::Navigation, "Branches"), |ui| {
        ui.clip(rect, |ui| match mode {
            RailMode::Icons => icons(ui, rect, intents),
            RailMode::Full | RailMode::Hidden => names(ui, (rect, state.unfolded), intents),
        });
    });
}

/// The rail with the names of the branches.
fn names(ui: &mut Ui<'_>, (rect, unfolded): (Rect, [bool; 3]), intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let inner = inset(rect, Insets::symmetric(spacing.large, spacing.large + spacing.small));
    let mut area = Area::with_gap(inner, spacing.large + spacing.small + spacing.tiny);
    let yours = area.top(TITLE + BRANCH_ROW * 4.0);
    let mut section = Area::new(yours);
    title(ui, section.top(TITLE), "YOURS");
    your_branches(ui, (section.rest(), RailMode::Full), intents);
    let origin = area.top(TITLE + QUIET_ROW * 3.0);
    let mut section = Area::new(origin);
    title(ui, section.top(TITLE), "ORIGIN");
    remote_branches(ui, (section.rest(), RailMode::Full));
    let mut section = Area::new(area.rest());
    title(ui, section.top(TITLE), "KEPT ASIDE");
    for (index, is_unfolded) in unfolded.into_iter().enumerate() {
        kept(ui, (&mut section, index), is_unfolded, intents);
    }
}

/// The rail with icons only.
fn icons(ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let inner = inset(
        rect,
        Insets::symmetric(spacing.small + spacing.tiny, spacing.large + spacing.small),
    );
    let mut area = Area::with_gap(inner, spacing.large);
    your_branches(ui, (area.top(BRANCH_ROW * 4.0), RailMode::Icons), intents);
    ui.separator(area.top(theme.sizes.border), Axis::Horizontal);
    remote_branches(ui, (area.top(QUIET_ROW * 3.0), RailMode::Icons));
    ui.separator(area.top(theme.sizes.border), Axis::Horizontal);
    let marks = [Icon::Stash, Icon::Worktree, Icon::Tag];
    for (icon, (name, _)) in marks.into_iter().zip(KEPT) {
        let side = theme.sizes.control_height;
        let cell = area.top(side);
        let rect = place(cell, (side, side), (Align::Center, Align::Start));
        let button = Button::icon(icon, name).kind(ButtonKind::Icon);
        if ui.button(("kept icon", name), rect, &button).clicked {
            intents.push(GalleryIntent::Clicked(name.to_owned()));
        }
    }
}

/// The title of a section of the rail.
fn title(ui: &mut Ui<'_>, rect: Rect, text: &str) {
    let padded = inset(rect, Insets::symmetric(ui.theme().sizes.spacing.medium, 0.0));
    let shown = Label::new(text).kind(TextKind::SmallStrong).tone(Tone::Muted);
    let _ = ui.label(padded, &shown);
}

/// The name of a branch of the rail, as find in page and the screen reader see it.
fn branch_text(row: usize) -> String {
    BRANCHES.get(row).map_or_default(|branch| branch.name.to_owned())
}

/// The name of a remote branch, as find in page and the screen reader see it.
fn remote_text(row: usize) -> String {
    REMOTES.get(row).map_or_default(|(name, _)| (*name).to_owned())
}

/// Your branches: a list with the dot of each lane, the name, and how it stands; a double click or Enter switches to
/// one, and its context menu has what can be done with it.
fn your_branches(ui: &mut Ui<'_>, (rect, mode): (Rect, RailMode), intents: &mut Vec<GalleryIntent>) {
    let list = List {
        name: "Your branches",
        row_count: BRANCHES.len(),
        row_height: BRANCH_ROW,
        selection_mode: SelectionMode::Single,
        lead: 0.0,
    };
    let search = ListSearch {
        row_text: &branch_text,
        generation: 0,
    };
    let answer = ui.searchable_list("your branches", (rect, &list), &search, |ui, row| {
        branch_row(ui, row, mode);
    });
    let entries = menus::branch_menu();
    let cursor = ui
        .memory_view()
        .list_selection(widget_id("your branches"))
        .and_then(Selection::cursor);
    if let Some(row) = answer.activated {
        menus::run_command(ui, MenuCommand::SwitchBranch, Some(row), intents);
    }
    if let Some(command) = ui.context_menu("branch menu", &answer.response, &entries) {
        menus::run_command(ui, command, answer.context_row.or(cursor), intents);
    }
}

/// A row of your branches: the dot of its lane, and with the names, the name and how it stands.
fn branch_row(ui: &mut Ui<'_>, row: &Row, mode: RailMode) {
    let Some(branch) = BRANCHES.get(row.index) else {
        return;
    };
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let color = theme.lane(branch.lane);
    if mode == RailMode::Icons {
        ui.fill_circle(row.rect.center(), DOT * 0.5, color);
        return;
    }
    let mut parts = Area::with_gap(
        inset(row.rect, Insets::symmetric(spacing.large, 0.0)),
        spacing.medium + 2.0,
    );
    let dot = parts.left(DOT);
    ui.fill_circle(dot.center(), DOT * 0.5, color);
    let state = Label::new(branch.state).kind(TextKind::Small).tone(Tone::Muted);
    let state_width = ui.measure(branch.state, ui.text_style(TextKind::Small));
    let _ = ui.draw_label(parts.right(state_width), &state);
    let kind = if row.index == 0 {
        TextKind::Strong
    } else {
        TextKind::Medium
    };
    let name = Label::new(branch.name).kind(kind);
    let _ = ui.draw_label(parts.rest(), &name);
}

/// The remote branches of origin: quieter rows, with a ring in the color of the lane of their branch.
fn remote_branches(ui: &mut Ui<'_>, (rect, mode): (Rect, RailMode)) {
    let list = List {
        name: "Remote branches",
        row_count: REMOTES.len(),
        row_height: QUIET_ROW,
        selection_mode: SelectionMode::Single,
        lead: 0.0,
    };
    let search = ListSearch {
        row_text: &remote_text,
        generation: 0,
    };
    let _ = ui.searchable_list("remote branches", (rect, &list), &search, |ui, row| {
        remote_row(ui, row, mode);
    });
}

/// A row of the remote branches.
fn remote_row(ui: &mut Ui<'_>, row: &Row, mode: RailMode) {
    let Some((name, lane)) = REMOTES.get(row.index) else {
        return;
    };
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let color = lane.map_or(theme.colors.muted_foreground, |lane| theme.lane(lane));
    let ring = Stroke {
        width: theme.sizes.graph_line,
        cap: LineCap::Butt,
        color,
    };
    let radius = (DOT - theme.sizes.graph_line) * 0.5;
    if mode == RailMode::Icons {
        ui.stroke_circle(row.rect.center(), radius, ring);
        return;
    }
    let mut parts = Area::with_gap(
        inset(row.rect, Insets::symmetric(spacing.large, 0.0)),
        spacing.medium + 2.0,
    );
    let dot = parts.left(DOT);
    ui.stroke_circle(dot.center(), radius, ring);
    let _ = ui.draw_label(parts.rest(), &Label::new(name).tone(Tone::Muted));
}

/// A section of what is kept aside: a row with its name and how many it holds, which unfolds the first of them under
/// it.
fn kept(ui: &mut Ui<'_>, (area, index): (&mut Area, usize), is_unfolded: bool, intents: &mut Vec<GalleryIntent>) {
    let Some((name, entries)) = KEPT.get(index) else {
        return;
    };
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let rect = area.top(QUIET_ROW);
    let id = ui.id(("kept", index));
    let response = ui.interact(id, rect, Sense::CLICK | Sense::FOCUS);
    if response.hovered {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    if response.shows_focus {
        ui.focus_ring(rect, theme.sizes.radius.medium);
    }
    if response.clicked {
        intents.push(GalleryIntent::Unfold(index));
    }
    let count = entries.len().to_string();
    let mut parts = Area::with_gap(inset(rect, Insets::symmetric(spacing.medium, 0.0)), spacing.small);
    let chevron = if is_unfolded {
        Icon::ChevronDown
    } else {
        Icon::ChevronRight
    };
    ui.icon_in(parts.left(theme.sizes.icon), chevron, Tone::Muted);
    let count_width = ui.measure(&count, ui.text_style(TextKind::Body));
    let _ = ui.draw_label(parts.right(count_width), &Label::new(&count).tone(Tone::Muted));
    let _ = ui.draw_label(parts.rest(), &Label::new(name));
    let description = Description::new(AccessRole::Button, name)
        .value(&count)
        .expanded(is_unfolded);
    ui.describe(id, rect, &description);
    if is_unfolded {
        kept_entries(ui, area, entries);
    }
}

/// The first entries of an unfolded section, and how many more it holds.
fn kept_entries(ui: &mut Ui<'_>, area: &mut Area, entries: &[&str]) {
    let theme = *ui.theme();
    let indent = theme.sizes.spacing.medium + theme.sizes.icon + theme.sizes.spacing.small;
    let more = entries.len().saturating_sub(SHOWN_ENTRIES);
    let more_text = format!("{more} more");
    let shown = entries.iter().take(SHOWN_ENTRIES).copied();
    let all = shown.chain((more > 0).then_some(more_text.as_str()));
    for entry in all {
        let row = area.top(theme.sizes.row_height);
        let text = Rect::new(row.x + indent, row.y, (row.width - indent).max(0.0), row.height);
        let _ = ui.label(text, &Label::new(entry).kind(TextKind::Detail).tone(Tone::Muted));
    }
}
