//! The menu of the view, right before **…** at the end of the header (`graph.view-menu`): every switch of the
//! timeline, each checked while it is on, in three groups: what the timeline shows, how it draws it, and the columns
//! of each row (D66).

use model::filter::{Kinds, ReferenceKind};
use ui::{Button, ButtonKind, MenuEntry, Ui};

use super::fold::{FoldAllEntry, FoldAllState, fold_all_view};
use super::intent::{GraphIntent, Switches, TimelineSwitch};
use super::switches::{FirstParentState, SelectedTrace, first_parent_view, trace_selected_view};
use crate::icons;
use crate::intent::{FilterIntent, Intent};
use crate::text::filter as filter_words;
use crate::text::header as words;

/// The key of the menu.
const VIEW_MENU: &str = "view menu";

/// What the timeline shows, after the kinds of references: the stashes, and only the first parent of merges.
const SHOWN: [TimelineSwitch; 2] = [TimelineSwitch::ShowStashes, TimelineSwitch::FirstParent];

/// How the timeline draws its rows, before the folds: **Compact rows** (D65) right after one line.
const DRAWN: [TimelineSwitch; 5] = [
    TimelineSwitch::OneLane,
    TimelineSwitch::CompactRows,
    TimelineSwitch::TraceSelected,
    TimelineSwitch::DateSeparators,
    TimelineSwitch::CombineUpstream,
];

/// What each row shows beside its message (`graph.columns`).
const COLUMNS: [TimelineSwitch; 3] = [TimelineSwitch::Author, TimelineSwitch::Date, TimelineSwitch::Hash];

/// What the menu of the view offers: the switches of the timeline, once a repository is read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ViewMenuState {
    /// The switches of the timeline and its folds; with none, the button cannot be clicked.
    pub timeline: Option<TimelineMenu>,
}

/// The switches of the timeline, as they stand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimelineMenu {
    /// The switches, as they stand.
    pub switches: Switches,
    /// Whether a merge that can be folded is loaded.
    pub has_merges: bool,
    /// Whether a merge is folded.
    pub has_folds: bool,
    /// Whether every merge is folded.
    pub is_all_folded: bool,
    /// How the timeline is drawn: the folds are offered only while it has its lines.
    pub drawn: GraphDrawn,
    /// The kinds of references the timeline shows; none while the branch filter chose otherwise (`filter.kinds`).
    pub kinds: Option<Kinds>,
}

/// How the timeline is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphDrawn {
    /// With the lines of the graph.
    WithLines,
    /// As a list of what a search finds, where nothing is folded.
    AsList,
}

/// What an item of the menu does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ViewCommand {
    /// A switch of the timeline.
    Switch(TimelineSwitch),
    /// **Fold all merges** or **Unfold all merges**.
    FoldAll(FoldAllEntry),
    /// **Show local branches**, **Show remote branches** or **Show tags**.
    Kind(ReferenceKind),
}

/// The words of a switch.
fn switch_words(switch: TimelineSwitch) -> &'static str {
    match switch {
        TimelineSwitch::ShowStashes => words::SHOW_STASHES,
        TimelineSwitch::FirstParent => words::FIRST_PARENT,
        TimelineSwitch::OneLane => words::ONE_LANE,
        TimelineSwitch::TraceSelected => words::TRACE_SELECTED,
        TimelineSwitch::Author => words::SHOW_AUTHOR,
        TimelineSwitch::Date => words::SHOW_DATE,
        TimelineSwitch::Hash => words::SHOW_HASH,
        TimelineSwitch::DateSeparators => words::DATE_SEPARATORS,
        TimelineSwitch::CombineUpstream => words::COMBINE_UPSTREAM,
        TimelineSwitch::CompactRows => words::COMPACT_ROWS,
    }
}

/// The items of some switches, each with a check while it is on.
fn switch_items(switches: Switches, group: &[TimelineSwitch]) -> impl Iterator<Item = MenuEntry<'static, ViewCommand>> {
    group.iter().map(move |&switch| {
        MenuEntry::Item(icons::choice(
            switch_words(switch),
            ViewCommand::Switch(switch),
            switches.is_on(switch),
        ))
    })
}

/// **Fold all merges**, checked while every merge is folded, and **Unfold all merges**, while the graph has its lines
/// and is not on one line; each greyed out while it has nothing to do.
fn fold_entries(menu: TimelineMenu) -> Vec<MenuEntry<'static, ViewCommand>> {
    if menu.drawn != GraphDrawn::WithLines || menu.switches.is_on(TimelineSwitch::OneLane) {
        return Vec::new();
    }
    let fold = icons::choice(
        words::FOLD_ALL,
        ViewCommand::FoldAll(FoldAllEntry::Fold),
        menu.is_all_folded,
    );
    let unfold = icons::action(
        icons::UNFOLD,
        words::UNFOLD_ALL,
        ViewCommand::FoldAll(FoldAllEntry::Unfold),
    );
    vec![
        MenuEntry::Item(if menu.is_all_folded || menu.has_merges {
            fold
        } else {
            fold.disabled()
        }),
        MenuEntry::Item(if menu.has_folds { unfold } else { unfold.disabled() }),
    ]
}

/// The kinds of references shown, each checked while it is shown: greyed out with **Chosen in the filter** while the
/// branch filter chose otherwise, and the one kind left on greyed out, since one is always on.
pub(crate) fn kind_entries(kinds: Option<Kinds>) -> Vec<MenuEntry<'static, ViewCommand>> {
    [
        (ReferenceKind::Local, filter_words::SHOW_LOCAL),
        (ReferenceKind::Remote, filter_words::SHOW_REMOTE),
        (ReferenceKind::Tag, filter_words::SHOW_TAGS),
    ]
    .into_iter()
    .map(|(kind, label)| {
        let is_shown = kinds.is_some_and(|kinds| kinds.has(kind));
        let item = icons::choice(label, ViewCommand::Kind(kind), is_shown);
        MenuEntry::Item(match kinds {
            None => item.disabled().with_shortcut(filter_words::CHOSEN_IN_FILTER),
            Some(kinds) if kinds.has(kind) && kinds.count() == 1 => item.disabled(),
            Some(_) => item,
        })
    })
    .collect()
}

/// The entries of the menu, in the groups of the card: what the timeline shows, how it draws it with its folds, and
/// the columns of each row.
fn menu_entries(menu: TimelineMenu) -> Vec<MenuEntry<'static, ViewCommand>> {
    let mut entries = kind_entries(menu.kinds);
    entries.extend(switch_items(menu.switches, &SHOWN));
    entries.push(MenuEntry::Separator);
    entries.extend(switch_items(menu.switches, &DRAWN));
    entries.extend(fold_entries(menu));
    entries.push(MenuEntry::Separator);
    entries.extend(switch_items(menu.switches, &COLUMNS));
    entries
}

/// The button of the menu of the view and its menu, over the rectangle its caller clipped it to. The button cannot be
/// clicked before a repository is read.
///
/// Feature cards that fill it further: `filter.kinds`, `graph.stashes`, `graph.first-parent`, `graph.one-lane`,
/// `graph.trace-selected`, `graph.date-separators`, `graph.combine-upstream`, `graph.fold-all`, `graph.columns`.
///
/// Feature: graph.view-menu
pub fn view_menu_view(state: &ViewMenuState, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let mut button = Button::icon(icons::VIEW_MENU, words::VIEW).kind(ButtonKind::Outline);
    if state.timeline.is_none() {
        button = button.disabled();
    }
    let entries = state.timeline.map(menu_entries).unwrap_or_default();
    let switches = state.timeline.map_or(Switches::DEFAULT, |timeline| timeline.switches);
    match ui.menu_button(VIEW_MENU, rect, &button, &entries) {
        Some(ViewCommand::Kind(kind)) => vec![Intent::Filter(FilterIntent::SwitchKind(kind))],
        Some(ViewCommand::Switch(TimelineSwitch::FirstParent)) => {
            let state = FirstParentState {
                is_on: switches.is_on(TimelineSwitch::FirstParent),
            };
            first_parent_view(&state, ui)
        }
        Some(ViewCommand::Switch(TimelineSwitch::TraceSelected)) => {
            let state = SelectedTrace {
                is_on: switches.is_on(TimelineSwitch::TraceSelected),
            };
            trace_selected_view(&state, ui)
        }
        Some(ViewCommand::Switch(switch)) => vec![Intent::Graph(GraphIntent::Switch(switch, !switches.is_on(switch)))],
        Some(ViewCommand::FoldAll(chosen)) => {
            let state = FoldAllState {
                chosen,
                is_all_folded: state.timeline.is_some_and(|timeline| timeline.is_all_folded),
            };
            fold_all_view(&state, ui)
        }
        None => Vec::new(),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
