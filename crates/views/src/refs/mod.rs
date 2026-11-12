//! The branch rail, on the left of the window (`refs.tree`): the local branches, each with the dot of its lane, the
//! branches of each remote under its name, and **Kept aside**: the stashes, the worktrees and the tags, in lists that
//! open and close. In the narrow layout it shows only the dots and the icons.

mod rows;

use std::collections::HashMap;

use model::state::RepositoryState;
use ui::layout::{Align, Insets, inset, place};
use ui::{AccessRole, Axis, Description, Icon, Rect, Tone, Ui};

use crate::intent::{Intent, KeptList};
use crate::line::is_narrow;
use crate::text::rail as words;
use rows::{RailRow, rail_rows};

/// What the branch rail lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReferencesState<'a> {
    /// The repository shown, as last read.
    pub repository: &'a RepositoryState,
    /// The lists of **Kept aside** that are open; they start closed.
    pub open: &'a [KeptList],
    /// The color of the line of each local branch in the timeline, by its name; a branch whose commit is not shown has
    /// none, and its dot is muted.
    pub branch_colors: &'a HashMap<String, usize>,
}

/// The branch rail, over the rectangle its caller clipped it to.
///
/// Feature cards that fill it: `refs.tree` (a click on a branch narrows the timeline to its history, a click on a tag,
/// a stash or a worktree goes to its commit, the menus of each reference, **Favorites** and **Recent**, the
/// submodules, **Show all…** past 100 entries), `refs.filter`, `refs.scope-line`, `refs.sort`, `refs.favorite`,
/// `graph.toggle-references`. The dot of a branch is in the color of its line in the timeline, as its labels are.
pub fn tree_view(state: &ReferencesState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let rect = ui.current_clip();
    let theme = *ui.theme();
    ui.fill_rect(rect, theme.colors.rail_background);
    let edge = Rect::new(
        rect.right() - theme.sizes.border,
        rect.y,
        theme.sizes.border,
        rect.height,
    );
    ui.separator(edge, Axis::Vertical);
    let is_icons = is_narrow(ui);
    let rows = rail_rows(state, is_icons);
    let spacing = theme.sizes.spacing;
    let across = if is_icons {
        spacing.small + spacing.tiny
    } else {
        spacing.large
    };
    let inner = inset(rect, Insets::symmetric(across, spacing.large + spacing.small));
    let height: f32 = rows.iter().map(|row| row.height(&theme)).sum();
    ui.region("rail region", (rect, AccessRole::Navigation, words::RAIL), |ui| {
        let _ = ui.scroll_area("rail", inner, (inner.width, height), |ui, content| {
            let mut y = content.y;
            for row in &rows {
                let row_height = row.height(&theme);
                let row_rect = Rect::new(content.x, y, content.width, row_height);
                y += row_height;
                if row_rect.bottom() < inner.y || row_rect.y > inner.bottom() {
                    continue;
                }
                if let Some(intent) = draw_row(ui, row, row_rect, is_icons) {
                    intents.push(intent);
                }
            }
        });
    });
    intents
}

/// Draws a row of the rail, and says what a click on it asked for.
fn draw_row(ui: &mut Ui<'_>, row: &RailRow<'_>, rect: Rect, is_icons: bool) -> Option<Intent> {
    match row {
        RailRow::Title(title) => {
            rows::title(ui, rect, title);
            None
        }
        RailRow::Branch { color, branch } => {
            rows::branch(ui, rect, (*color, branch), is_icons);
            None
        }
        RailRow::Remote(branch) => {
            rows::remote(ui, rect, branch, is_icons);
            None
        }
        RailRow::Kept { list, count, .. } if is_icons => {
            kept_icon(ui, rect, (*list, *count));
            None
        }
        RailRow::Kept { list, count, is_open } => rows::kept(ui, rect, (*list, *count, *is_open)),
        RailRow::Entry(text) => {
            rows::entry(ui, rect, text);
            None
        }
        RailRow::Gap => None,
    }
}

/// The icon of a list of **Kept aside**, in the narrow layout.
fn kept_icon(ui: &mut Ui<'_>, rect: Rect, (list, count): (KeptList, usize)) {
    let side = ui.theme().sizes.icon;
    let cell = place(rect, (side, side), (Align::Center, Align::Center));
    ui.icon_in(cell, kept_icon_of(list), Tone::Muted);
    let id = ui.id(("kept icon", list));
    ui.describe(
        id,
        cell,
        &Description::new(AccessRole::Label, rows::kept_name(list)).value(&count.to_string()),
    );
}

/// The icon of a list of **Kept aside**.
fn kept_icon_of(list: KeptList) -> Icon {
    match list {
        KeptList::Stashes => Icon::Stash,
        KeptList::Worktrees => Icon::Worktree,
        KeptList::Tags => Icon::Tag,
    }
}
