//! How an entry of the branch rail is drawn and what it does: a branch narrows the timeline to its history, the others
//! go to their commit; its menu, its tooltip, the eye that adds a dimmed branch to the timeline, and the history it
//! brings out under the pointer.

use std::time::Duration;

use model::filter::{ReferenceKind, show_only, show_too};
use model::state::{Branch, Worktree};
use ui::layout::{Area, Insets, inset};
use ui::{
    AccessRole, Button, ButtonKind, Description, Icon, Label, LineCap, MenuAnchor, Rect, Response, Sense, Stroke,
    TextKind, Tone, Ui,
};

use super::ReferencesState;
use super::draw::{Context, DOT, RailReport, indent};
use super::menu::{EntryCommand, entry_menu};
use super::rows::{Entry, Marks, worktree_folder};
use crate::filter::filter_entry_intent;
use crate::graph::{GraphIntent, HoverTarget, HoverWay};
use crate::intent::{FilterIntent, Intent, RailList};
use crate::text::header;
use crate::text::rail as words;

/// An entry of a list: drawn by its kind, with what a click, a right click and the pointer resting on it do.
pub(super) fn entry_row(
    ui: &mut Ui<'_>,
    context: Context<'_, '_>,
    (list, entry, marks): (&RailList, &Entry<'_>, Marks),
    rect: Rect,
    report: &mut RailReport,
) {
    let theme = *ui.theme();
    let name = entry.name();
    let id = ui.id(("entry", list, &name));
    let response = ui.interact(id, rect, Sense::CLICK);
    let radius = theme.sizes.radius.medium;
    if marks.is_selected && !context.is_icons {
        ui.fill_rounded_rect(rect, radius, theme.colors.list_inactive_selection);
    }
    if response.hovered && !context.is_icons {
        ui.fill_rounded_rect(rect, radius, theme.colors.list_hover);
    }
    // The eye takes the pointer from the row under it, so the row counts as pointed at while the pointer is anywhere
    // on it, the eye included.
    let is_pointed = response.hovered
        || ui
            .pointer()
            .is_some_and(|point| rect.contains(point) && ui.current_clip().contains(point));
    let eye = (marks.is_dimmed && is_pointed).then(|| {
        let side = rect.height;
        Rect::new(rect.right() - side, rect.y, side, rect.height)
    });
    let drawn = Rect::new(
        rect.x,
        rect.y,
        eye.map_or(rect.width, |eye| (eye.x - rect.x).max(0.0)),
        rect.height,
    );
    draw_entry(ui, context, (entry, marks), drawn);
    if let Some(eye) = eye {
        report.intents.extend(eye_button(ui, context.state, entry, eye));
    }
    let tooltip = entry_tooltip(context.state, entry, marks);
    if !tooltip.is_empty() {
        ui.tooltip(&response, &tooltip);
    }
    ui.describe(
        id,
        rect,
        &Description::new(AccessRole::Button, &name).selected(marks.is_selected),
    );
    if response.clicked {
        report.intents.extend(click_intents(context.state, entry));
    }
    if let Some(hover) = hover_of(ui, &response, entry) {
        report.hover = Some(hover);
    }
    let menu_key = ("entry menu", list, &name);
    if response.secondary_clicked {
        ui.open_menu(menu_key, MenuAnchor::Pointer);
    }
    if ui.is_menu_open(menu_key) {
        report
            .intents
            .extend(entry_menu_chosen(ui, context.state, entry, menu_key));
    }
}

/// The reference the pointer rests on, with since when, whose history the timeline brings out: what it reaches, or
/// with `Shift` what it has that the commit checked out has not.
fn hover_of(ui: &Ui<'_>, response: &Response, entry: &Entry<'_>) -> Option<(HoverTarget, Duration)> {
    let since = ui.hovered_since(response)?;
    let commit = entry.full_name().and(entry.commit())?.clone();
    let way = if ui.modifiers().shift() {
        HoverWay::NotCheckedOut
    } else {
        HoverWay::Reaches
    };
    Some((HoverTarget { commit, way }, since))
}

/// What a click on an entry asks: on a branch or a remote branch, the timeline narrowed to its history with its tip
/// selected, or everything again when it is the one shown alone; on a tag, a stash or a worktree, its commit.
pub(super) fn click_intents(state: &ReferencesState<'_>, entry: &Entry<'_>) -> Vec<Intent> {
    let commit = entry.commit().cloned();
    match (entry, entry.full_name()) {
        (Entry::Branch(_) | Entry::Remote(_), Some(full)) => {
            let only = show_only(&full);
            if state.shown == Some(&only) {
                return vec![Intent::Filter(FilterIntent::Show(None))];
            }
            let mut intents = vec![Intent::Filter(FilterIntent::Show(Some(only)))];
            intents.extend(commit.map(|commit| Intent::Graph(GraphIntent::GoTo(commit))));
            intents
        }
        _ => commit
            .map(|commit| Intent::Graph(GraphIntent::GoTo(commit)))
            .into_iter()
            .collect(),
    }
}

/// The menu of an entry while it is open, and what the entry chosen asks.
fn entry_menu_chosen(
    ui: &mut Ui<'_>,
    state: &ReferencesState<'_>,
    entry: &Entry<'_>,
    menu_key: (&str, &RailList, &String),
) -> Vec<Intent> {
    let check_out = words::check_out(&entry.name());
    let entries = entry_menu(state, entry, &check_out);
    let Some(chosen) = ui.menu(menu_key, &entries) else {
        return Vec::new();
    };
    let full = entry.full_name().unwrap_or_default();
    match chosen {
        EntryCommand::Filter(filter) => vec![filter_entry_intent(filter, state.shown, &full, state.references)],
        EntryCommand::Favorite => {
            let favorites = super::Favorites {
                reference: &full,
                is_favorite: state.favorites.contains(&full),
            };
            super::favorite_view(&favorites, ui)
        }
        EntryCommand::ShowInTimeline => entry
            .commit()
            .map(|commit| Intent::Graph(GraphIntent::GoTo(commit.clone())))
            .into_iter()
            .collect(),
        EntryCommand::Later => Vec::new(),
    }
}

/// The eye at the end of a dimmed reference under the pointer, which adds its history to the timeline.
fn eye_button(ui: &mut Ui<'_>, state: &ReferencesState<'_>, entry: &Entry<'_>, rect: Rect) -> Vec<Intent> {
    let (Some(full), Some(shown)) = (entry.full_name(), state.shown) else {
        return Vec::new();
    };
    let button = Button::icon(Icon::Eye, words::SHOW_TOO).kind(ButtonKind::Icon);
    if ui.button(("eye", &full), rect, &button).clicked {
        vec![Intent::Filter(FilterIntent::Show(Some(show_too(
            shown,
            &full,
            state.references,
        ))))]
    } else {
        Vec::new()
    }
}

/// The tooltip of an entry: about the upstream and the worktree of a local branch, the name of a stash, and **Not
/// shown in the timeline** on a dimmed reference.
fn entry_tooltip(state: &ReferencesState<'_>, entry: &Entry<'_>, marks: Marks) -> String {
    let mut lines = Vec::new();
    match entry {
        Entry::Branch(branch) => {
            let upstream = branch.upstream.as_ref().map(|upstream| short(&upstream.reference));
            lines.push(words::upstream_of(&branch.name, upstream));
            if let Some(worktree) = other_worktree(state, branch) {
                lines.push(words::in_worktree(&worktree_folder(worktree), worktree.uncommitted));
            }
        }
        Entry::Stash(stash) => lines.push(words::stash_tooltip(stash.index, &stash.message)),
        _ => {}
    }
    if marks.is_dimmed {
        lines.push(words::NOT_SHOWN.to_owned());
    }
    lines.join("\n")
}

/// A full name without the prefix of its kind.
fn short(full: &str) -> &str {
    ReferenceKind::ALL
        .iter()
        .find_map(|kind| full.strip_prefix(kind.prefix()))
        .unwrap_or(full)
}

/// The worktree other than this window's that has a branch checked out.
fn other_worktree<'a>(state: &ReferencesState<'a>, branch: &Branch) -> Option<&'a Worktree> {
    let full = ReferenceKind::Local.full_name(&branch.name);
    state
        .repository
        .worktrees
        .iter()
        .find(|worktree| !worktree.is_current && worktree.branch.as_deref() == Some(full.as_str()))
}

/// Draws an entry by its kind.
fn draw_entry(ui: &mut Ui<'_>, context: Context<'_, '_>, (entry, marks): (&Entry<'_>, Marks), rect: Rect) {
    let theme = *ui.theme();
    let lane = |name: &str, colors: &std::collections::HashMap<String, usize>| colors.get(name).copied();
    match entry {
        Entry::Branch(branch) => {
            let color = lane(&branch.name, context.state.branch_colors);
            let color = color.map_or(theme.colors.muted_foreground, |color| theme.lane(color));
            if context.is_icons {
                ui.fill_circle(rect.center(), DOT * 0.5, color);
                return;
            }
            branch_row(ui, context.state, (branch, color, marks), rect);
        }
        Entry::Remote(branch) => {
            let color = lane(&branch.name, context.state.remote_colors);
            let color = color.map_or(theme.colors.muted_foreground, |color| theme.lane(color));
            let ring = Stroke {
                width: theme.sizes.graph_line,
                cap: LineCap::Butt,
                color,
            };
            let radius = (DOT - theme.sizes.graph_line) * 0.5;
            if context.is_icons {
                ui.stroke_circle(rect.center(), radius, ring);
                return;
            }
            let mut parts = name_parts(ui, rect, marks);
            let dot = parts.left(DOT);
            ui.stroke_circle(dot.center(), radius, ring);
            let tone = if marks.is_dimmed { Tone::Disabled } else { Tone::Muted };
            let _ = ui.label(parts.rest(), &Label::new(&branch.name).tone(tone));
        }
        _ => kept_entry(ui, entry, marks, rect),
    }
}

/// The room of the words of a dot row, with the star of a favorite at its end.
fn name_parts(ui: &mut Ui<'_>, rect: Rect, marks: Marks) -> Area {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut parts = Area::with_gap(
        inset(rect, Insets::symmetric(spacing.medium, 0.0)),
        spacing.medium + 2.0,
    );
    if marks.is_favorite {
        ui.icon_in(parts.right(theme.sizes.icon), Icon::Star, Tone::Warning);
    }
    parts
}

/// A local branch: the dot of its lane, its name, the worktree that has it, and how far it is from its upstream.
fn branch_row(
    ui: &mut Ui<'_>,
    state: &ReferencesState<'_>,
    (branch, color, marks): (&Branch, ui::Color, Marks),
    rect: Rect,
) {
    let theme = *ui.theme();
    let mut parts = name_parts(ui, rect, marks);
    let dot = parts.left(DOT);
    ui.fill_circle(dot.center(), DOT * 0.5, color);
    if let Some(stands) = &branch_stands(branch) {
        let width = ui.measure(stands, ui.text_style(TextKind::Small));
        let _ = ui.label(
            parts.right(width),
            &Label::new(stands).kind(TextKind::Small).tone(Tone::Muted),
        );
    }
    if let Some(worktree) = other_worktree(state, branch) {
        let files = worktree
            .uncommitted
            .filter(|files| *files > 0)
            .map(|files| files.to_string());
        if let Some(files) = &files {
            let width = ui.measure(files, ui.text_style(TextKind::Small));
            let _ = ui.label(
                parts.right(width),
                &Label::new(files).kind(TextKind::Small).tone(Tone::Muted),
            );
        }
        ui.icon_in(parts.right(theme.sizes.icon), Icon::Worktree, Tone::Muted);
    }
    let kind = if branch.is_checked_out {
        TextKind::Strong
    } else {
        TextKind::Medium
    };
    let tone = if marks.is_dimmed { Tone::Disabled } else { Tone::Normal };
    let _ = ui.label(parts.rest(), &Label::new(&branch.name).kind(kind).tone(tone));
}

/// How far a local branch is from its upstream, `↑1 ↓2`, or `gone` when its upstream was deleted; none without one.
pub(super) fn branch_stands(branch: &Branch) -> Option<String> {
    branch.upstream.as_ref().map(|upstream| {
        if upstream.is_gone {
            words::GONE.to_owned()
        } else {
            header::upstream_distance(upstream.ahead, upstream.behind)
        }
    })
}

/// An entry of a list kept aside, or of the tags: its icon and its name under the title of its list.
fn kept_entry(ui: &mut Ui<'_>, entry: &Entry<'_>, marks: Marks, rect: Rect) {
    let theme = *ui.theme();
    let indent = indent(&theme);
    let shown = Rect::new(rect.x + indent, rect.y, (rect.width - indent).max(0.0), rect.height);
    let mut parts = Area::with_gap(shown, theme.sizes.spacing.small);
    let icon = match entry {
        Entry::Tag(_) => Icon::Tag,
        Entry::Stash(_) => Icon::Stash,
        Entry::Worktree(_) => Icon::Worktree,
        _ => Icon::Folder,
    };
    ui.icon_in(parts.left(theme.sizes.icon), icon, Tone::Muted);
    let text = match entry {
        Entry::Worktree(worktree) => worktree_words(worktree),
        _ => entry.name(),
    };
    let kind = if entry.is_current() {
        TextKind::Strong
    } else {
        TextKind::Detail
    };
    let tone = if marks.is_dimmed { Tone::Disabled } else { Tone::Muted };
    let _ = ui.label(parts.rest(), &Label::new(&text).kind(kind).tone(tone));
}

/// A worktree as the rail lists it: its folder, then its branch, `detached` or `missing`.
fn worktree_words(worktree: &Worktree) -> String {
    let stands = if worktree.is_missing {
        words::MISSING
    } else {
        worktree.branch.as_deref().map_or(words::DETACHED, short)
    };
    format!("{} \u{b7} {stands}", worktree_folder(worktree))
}
