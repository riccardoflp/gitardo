//! The Submodules list of the branch rail, kept aside (`submodule.section`): its title with **+**, which adds one, and
//! each submodule indented under the one that has it, with where it stands against the commit recorded for it, the
//! branch it has checked out and how many of its files are not committed; a double click opens it in a new window, and
//! its menu has the operations of `submodule.`.
//!
//! Its rows are rows of the rail, so that the filter box and the drawing of the rows in sight work for it as for the
//! other lists.

use model::contract::{FeatureId, Target};
use model::submodule::{Submodule, SubmoduleStatus, status};
use ui::layout::{Align, Area, place};
use ui::{
    AccessRole, Button, ButtonKind, Description, Icon, Label, MenuAnchor, MenuEntry, Rect, Sense, TextKind, Tone, Ui,
};

use super::draw::{Context, indent, list_title};
use super::rows::{Entry, ListHead, RailRow};
use crate::branch::{OperationEntry, chosen};
use crate::feature::FeatureIntent;
use crate::icons;
use crate::intent::{Intent, RailList};
use crate::text::submodule as words;

/// A row of the Submodules list, and where it is drawn.
#[derive(Clone, Copy)]
pub(super) struct SubmodulesRow<'s, 'a> {
    /// What the rail is drawn from.
    pub(super) context: Context<'s, 'a>,
    /// The row.
    pub(super) row: &'s RailRow<'a>,
    /// Where it is, which may be partly out of sight.
    pub(super) rect: Rect,
}

/// A row of the Submodules list of the branch rail, over its rectangle: the title of the list with **+**, or a
/// submodule.
///
/// Feature: submodule.section
pub(super) fn section(state: &SubmodulesRow<'_, '_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let (context, rect) = (state.context, state.rect);
    match state.row {
        RailRow::Kept(head) | RailRow::Heading(head) => title(ui, context, head, rect),
        RailRow::Entry {
            entry: Entry::Submodule(submodule),
            ..
        } => submodule_row(ui, context, submodule, rect),
        // The other rows are not of the Submodules list, and the rail draws them itself.
        _ => Vec::new(),
    }
}

/// Whether a row is one of the Submodules list, which [`section`] draws.
pub(super) fn is_of_submodules(row: &RailRow<'_>) -> bool {
    match row {
        RailRow::Kept(head) | RailRow::Heading(head) => head.list == RailList::Submodules,
        RailRow::Entry { list, .. } => *list == RailList::Submodules,
        _ => false,
    }
}

/// The title of the list, with **+** at its end, which cannot be clicked while an operation runs.
fn title(ui: &mut Ui<'_>, context: Context<'_, '_>, head: &ListHead, rect: Rect) -> Vec<Intent> {
    let side = ui.theme().sizes.row_height;
    let mut parts = Area::new(rect);
    let cell = parts.right(side + ui.theme().sizes.spacing.small);
    let mut intents = list_title(ui, context, head, parts.rest());
    let mut button = Button::icon(icons::ADD, words::ADD).kind(ButtonKind::Icon);
    if context.state.stashes.is_operation_running {
        button = button.disabled();
    }
    let placed = place(cell, (side, side), (Align::Start, Align::Center));
    if ui.button("add submodule", placed, &button).clicked {
        intents.push(run("submodule.add", Target::Head));
    }
    intents
}

/// What runs an operation on a target.
fn run(feature: &'static str, target: Target) -> Intent {
    Intent::Feature(FeatureIntent::Run(FeatureId(feature), target))
}

/// A submodule: lit under the pointer, with its tooltip; a double click opens it in a new window when it is
/// initialized, a right click opens its menu.
fn submodule_row(ui: &mut Ui<'_>, context: Context<'_, '_>, submodule: &Submodule, rect: Rect) -> Vec<Intent> {
    let theme = *ui.theme();
    let id = ui.id(("submodule", &submodule.folder));
    let response = ui.interact(id, rect, Sense::CLICK);
    if response.hovered {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    let all = &context.state.repository.submodules;
    draw_submodule(ui, (submodule, depth(all, submodule)), rect);
    ui.tooltip(&response, &tooltip(submodule));
    let summary = summary(submodule);
    ui.describe(
        id,
        rect,
        &Description::new(AccessRole::Button, &submodule.folder).value(&summary),
    );
    let mut intents = Vec::new();
    if response.double_clicked && submodule.checkout.is_some() {
        intents.push(run("submodule.open-in-new-window", target(submodule)));
    }
    let menu_key = ("submodule menu", &submodule.folder);
    if response.secondary_clicked {
        ui.open_menu(menu_key, MenuAnchor::Pointer);
    }
    if ui.is_menu_open(menu_key) {
        let groups = menu_groups(submodule, std::env::consts::OS);
        let operations: Vec<OperationEntry> = groups.iter().flatten().cloned().collect();
        if let Some(index) = ui.menu(menu_key, &menu_entries(&groups)) {
            intents.extend(chosen(&operations, index));
        }
    }
    intents
}

/// The target of the operations of a submodule.
fn target(submodule: &Submodule) -> Target {
    Target::Submodule(submodule.folder.clone())
}

/// How many submodules have this one inside them.
fn depth(all: &[Submodule], submodule: &Submodule) -> usize {
    let mut depth = 0;
    let mut parent = submodule.parent.as_deref();
    while let Some(folder) = parent {
        depth += 1;
        parent = all
            .iter()
            .find(|other| other.folder == folder)
            .and_then(|other| other.parent.as_deref())
            .filter(|_| depth < all.len());
    }
    depth
}

/// The name a submodule is listed by: its folder, from the submodule that has it for one inside another.
pub(super) fn shown_name(submodule: &Submodule) -> &str {
    submodule
        .parent
        .as_deref()
        .and_then(|parent| submodule.folder.strip_prefix(parent))
        .and_then(|rest| rest.strip_prefix('/'))
        .unwrap_or(&submodule.folder)
}

/// Draws a submodule, indented one step for each submodule that has it: its icon and its name, then the branch it has
/// checked out, the pencil with the files not committed and where it stands, at the end.
fn draw_submodule(ui: &mut Ui<'_>, (submodule, depth): (&Submodule, usize), rect: Rect) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let step = theme.sizes.icon + spacing.small;
    #[allow(clippy::cast_precision_loss, reason = "submodules are not nested millions deep")]
    let start = indent(&theme) + step * depth as f32;
    let shown = Rect::new(
        rect.x + start,
        rect.y,
        (rect.width - start - spacing.medium).max(0.0),
        rect.height,
    );
    let mut parts = Area::with_gap(shown, spacing.small);
    ui.icon_in(parts.left(theme.sizes.icon), icons::SUBMODULE, Tone::Muted);
    for (count, icon, tone) in status_marks(&status(submodule)).into_iter().rev() {
        ui.icon_in(parts.right(theme.sizes.icon), icon, tone);
        if let Some(count) = count {
            small(ui, &mut parts, &count.to_string(), tone);
        }
    }
    if let Some(checkout) = &submodule.checkout {
        if checkout.uncommitted > 0 {
            small(ui, &mut parts, &checkout.uncommitted.to_string(), Tone::Muted);
            ui.icon_in(parts.right(theme.sizes.icon), icons::EDITED, Tone::Muted);
        }
        small(
            ui,
            &mut parts,
            checkout.branch.as_deref().unwrap_or(words::DETACHED),
            Tone::Muted,
        );
    }
    let _ = ui.label(parts.rest(), &Label::new(shown_name(submodule)).kind(TextKind::Detail));
}

/// Small words at the end of what is left of a row.
fn small(ui: &mut Ui<'_>, parts: &mut Area, text: &str, tone: Tone) {
    let width = ui.measure(text, ui.text_style(TextKind::Small));
    let _ = ui.label(parts.right(width), &Label::new(text).kind(TextKind::Small).tone(tone));
}

/// The icons of where a submodule stands, from left to right, each after its number when it has one: `2↑ 1↓` when it
/// is both ahead and behind.
pub(super) fn status_marks(status: &SubmoduleStatus) -> Vec<(Option<u32>, Icon, Tone)> {
    match status {
        SubmoduleStatus::NotInitialized => vec![(None, icons::NOT_INITIALIZED, Tone::Muted)],
        SubmoduleStatus::Added => vec![(None, icons::EDITED, Tone::Muted)],
        SubmoduleStatus::Unknown { .. } => vec![(None, icons::WARNING, Tone::Warning)],
        SubmoduleStatus::Both { distance, .. } => vec![
            (Some(distance.ahead), icons::TO_PUSH, Tone::Warning),
            (Some(distance.behind), icons::TO_PULL, Tone::Warning),
        ],
        SubmoduleStatus::Ahead { commits, .. } => vec![(Some(*commits), icons::TO_PUSH, Tone::Warning)],
        SubmoduleStatus::Behind { commits, .. } => vec![(Some(*commits), icons::TO_PULL, Tone::Warning)],
        SubmoduleStatus::OnIt { .. } => vec![(None, icons::CHOSEN, Tone::Success)],
    }
}

/// What the row of a submodule says past its name, for the screen reader: the branch it has checked out, how many of
/// its files are not committed, and where it stands.
fn summary(submodule: &Submodule) -> String {
    let mut parts = Vec::new();
    if let Some(checkout) = &submodule.checkout {
        parts.push(checkout.branch.clone().unwrap_or_else(|| words::DETACHED.to_owned()));
        if checkout.uncommitted > 0 {
            parts.push(words::uncommitted(checkout.uncommitted));
        }
    }
    parts.push(status(submodule).sentence());
    parts.join(", ")
}

/// The tooltip of a submodule: its folder, its URL, where it stands, what it has checked out, the commit staged for it
/// when it is not the recorded one, and how many of its files are not committed.
pub(super) fn tooltip(submodule: &Submodule) -> String {
    let mut lines = vec![submodule.folder.clone()];
    lines.extend(submodule.url.clone());
    lines.push(status(submodule).sentence());
    if let Some(checkout) = &submodule.checkout {
        let short = checkout.commit.as_ref().map(model::CommitId::short);
        lines.push(words::checked_out(checkout.branch.as_deref(), short));
    }
    if let Some(staged) = &submodule.staged {
        lines.push(words::staged(staged.short()));
    }
    if let Some(files) = submodule
        .checkout
        .as_ref()
        .map(|checkout| checkout.uncommitted)
        .filter(|files| *files > 0)
    {
        lines.push(words::uncommitted(files));
    }
    lines.join("\n")
}

/// The operations of a submodule's menu in the groups of the card, those offered: opening it, bringing it to a commit,
/// copying, then taking it away. **Show in Gitardo** is greyed out until it is built.
pub(super) fn menu_groups(submodule: &Submodule, system: &str) -> Vec<Vec<OperationEntry>> {
    let on = || target(submodule);
    let runs = |label: &str, feature| OperationEntry::runs(label, feature, on());
    let checkout = submodule.checkout.as_ref();
    let is_initialized = checkout.is_some();
    let is_apart = matches!(
        status(submodule),
        SubmoduleStatus::Ahead { .. }
            | SubmoduleStatus::Behind { .. }
            | SubmoduleStatus::Both { .. }
            | SubmoduleStatus::Unknown { .. }
    );
    let mut opening = Vec::new();
    if is_initialized {
        opening.push(runs(words::OPEN_IN_NEW_WINDOW, "submodule.open-in-new-window"));
        opening.push(runs(words::OPEN_IN_TERMINAL, "submodule.open-in-terminal"));
        opening.push(runs(words::reveal(system), "submodule.reveal"));
        opening.push(runs(words::SHOW_IN_GITARDO, "submodule.show-in-gitardo"));
    }
    let mut moving = Vec::new();
    if !is_initialized {
        moving.push(runs(words::INITIALIZE, "submodule.initialize"));
    }
    if let (Some(recorded), true) = (&submodule.recorded, is_apart) {
        moving.push(runs(
            &words::update_to_recorded(recorded.short()),
            "submodule.update-to-recorded",
        ));
    }
    if let (Some(commit), true) = (checkout.and_then(|checkout| checkout.commit.as_ref()), is_apart) {
        moving.push(runs(&words::stage_recorded(commit.short()), "submodule.stage-recorded"));
    }
    if is_initialized {
        moving.push(runs(
            &words::update_to_latest(submodule.branch.as_deref()),
            "submodule.update-to-latest",
        ));
    }
    moving.push(runs(words::SYNC_URLS, "submodule.sync-urls"));
    let mut copying = vec![runs(words::COPY_PATH, "submodule.copy-path")];
    if submodule.url.is_some() {
        copying.push(runs(words::COPY_URL, "submodule.copy-url"));
    }
    let mut taking_away = Vec::new();
    if is_initialized {
        taking_away.push(runs(words::DEINITIALIZE, "submodule.deinitialize"));
    }
    taking_away.push(runs(words::REMOVE, "submodule.remove"));
    [opening, moving, copying, taking_away]
        .into_iter()
        .filter(|group| !group.is_empty())
        .collect()
}

/// The entries of the menu, a separator between the groups, each with its place among all of them.
fn menu_entries(groups: &[Vec<OperationEntry>]) -> Vec<MenuEntry<'_, usize>> {
    let mut entries = Vec::new();
    let mut index = 0;
    for group in groups {
        if index > 0 {
            entries.push(MenuEntry::Separator);
        }
        for operation in group {
            let items = crate::branch::menu_items(std::slice::from_ref(operation), |_| index);
            entries.extend(items);
            index += 1;
        }
    }
    entries
}
