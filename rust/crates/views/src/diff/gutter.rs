//! The buttons on the band of a change in the gutter of a Diff page: **+** stages it (`diff.stage-change`), **−**
//! unstages it (`diff.unstage-change`), **⟲** reverts it (`diff.revert-change`) as an edit of the text of the working
//! tree side, saved at once. While the rows follow an edit and the diff is not computed again, there are no buttons.

use model::contract::{FeatureId, Target};
use model::diff::{DiffChange, DiffSide};
use ui::layout::{Align, Insets, inset, place};
use ui::{AccessRole, Description, Icon, Rect, Sense, Ui};

use super::{DiffPageState, DiffText, DiffVersion};
use crate::feature::FeatureIntent;
use crate::icons;
use crate::intent::Intent;
use crate::text::diff as words;

/// The side of the icon of a button of the gutter.
const ICON: f32 = 12.0;

/// What a button of the gutter does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GutterButton {
    /// **Stage this change**.
    Stage,
    /// **Unstage this change**, with the tooltip of a change of the working tree staged already when it is one.
    Unstage {
        /// Whether the change is of the working tree and staged already.
        is_staged: bool,
    },
    /// **Revert this change**.
    Revert,
}

impl GutterButton {
    /// Its operation.
    fn feature(self) -> FeatureId {
        match self {
            GutterButton::Stage => FeatureId("diff.stage-change"),
            GutterButton::Unstage { .. } => FeatureId("diff.unstage-change"),
            GutterButton::Revert => FeatureId("diff.revert-change"),
        }
    }

    /// Its icon.
    pub(crate) fn icon(self) -> Icon {
        match self {
            GutterButton::Stage => icons::ADD,
            GutterButton::Unstage { .. } => icons::UNSTAGE,
            GutterButton::Revert => icons::UNDO,
        }
    }

    /// Its name, which its tooltip says.
    fn name(self) -> &'static str {
        match self {
            GutterButton::Stage => words::STAGE_CHANGE,
            GutterButton::Unstage { is_staged: true } => words::STAGED_UNSTAGE_CHANGE,
            GutterButton::Unstage { is_staged: false } => words::UNSTAGE_CHANGE,
            GutterButton::Revert => words::REVERT_CHANGE,
        }
    }
}

/// The buttons a change offers, from the versions of the page: **+** for a change of the working tree against `HEAD`
/// or the index that is not staged, **−** for a change staged of the working tree against `HEAD` and for every change
/// of the index against `HEAD`, neither for a renamed file; **⟲** for a change of the working tree. None for a file in
/// conflict, whose base is on the left.
fn offered(state: &DiffPageState<'_>, (file, text, change): (usize, &DiffText, usize)) -> Vec<GutterButton> {
    let (left, right) = state.versions;
    if *left == DiffVersion::Base {
        // A file in conflict is staged or taken from one side whole, not change by change (`conflicts.`).
        return Vec::new();
    }
    let is_renamed = state.files.get(file).is_none_or(|paths| paths.left != paths.right);
    let is_staged = text.is_staged(change);
    let mut buttons = Vec::new();
    match (left, right) {
        (DiffVersion::Head | DiffVersion::Index, DiffVersion::WorkingTree) if !is_renamed => {
            if is_staged && *left == DiffVersion::Head {
                buttons.push(GutterButton::Unstage { is_staged: true });
            } else if !is_staged {
                buttons.push(GutterButton::Stage);
            }
        }
        (DiffVersion::Head, DiffVersion::Index) if !is_renamed => {
            buttons.push(GutterButton::Unstage { is_staged: false });
        }
        _ => {}
    }
    if *right == DiffVersion::WorkingTree {
        buttons.push(GutterButton::Revert);
    }
    buttons
}

/// The buttons of a change on the first row of its band in the gutter, side by side, clear of the line that is dragged
/// to make one side wider: pressing one runs its operation on the change and moves nothing.
pub(crate) fn change_buttons(
    ui: &mut Ui<'_>,
    state: &DiffPageState<'_>,
    (file, text, change): (usize, &DiffText, usize),
    gutter: Rect,
) -> Vec<Intent> {
    let buttons = offered(state, (file, text, change));
    if buttons.is_empty() || state.is_stale {
        return Vec::new();
    }
    let theme = *ui.theme();
    // The grip of the line between the left side and the gutter reaches this far into the gutter.
    let grip = theme.sizes.spacing.small + theme.sizes.border;
    let room = inset(
        gutter,
        Insets {
            left: grip,
            top: 0.0,
            right: theme.sizes.border,
            bottom: 0.0,
        },
    );
    let side = (room.width * 0.5).min(room.height);
    let mut intents = Vec::new();
    for (place_of, button) in buttons.iter().enumerate() {
        let cell = Rect::new(
            room.x + side * crate::number::float_of_count(place_of),
            room.y,
            side,
            room.height,
        );
        let rect = place(cell, (side, side), (Align::Center, Align::Center));
        if !gutter_button(ui, (state.id, file, change), *button, rect) {
            continue;
        }
        if *button == GutterButton::Revert {
            let revert = super::DiffEditIntent::Revert(file, change);
            intents.push(Intent::Diff(super::DiffIntent::Edit(state.id, revert)));
        } else {
            let target = Target::DiffChange(Box::new(diff_change(state, (file, text, change))));
            intents.push(Intent::Feature(FeatureIntent::Run(button.feature(), target)));
        }
    }
    intents
}

/// A small button of the gutter: its icon, a background while the pointer is on it, its tooltip and its name for the
/// screen reader. Whether it was clicked.
fn gutter_button(
    ui: &mut Ui<'_>,
    (page, file, change): (super::DiffPageId, usize, usize),
    button: GutterButton,
    rect: Rect,
) -> bool {
    let theme = *ui.theme();
    let id = ui.id(("diff gutter button", page, file, change, button.feature().0));
    let response = ui.interact(id, rect, Sense::CLICK | Sense::HOVER);
    let background = if response.hovered {
        theme.colors.list_hover
    } else {
        theme.colors.card_background
    };
    ui.fill_rounded_rect(rect, theme.sizes.radius.small, background);
    let icon = place(rect, (ICON, ICON), (Align::Center, Align::Center));
    ui.icon(
        button.icon(),
        ui::Point::new(icon.x, icon.y),
        ICON,
        theme.colors.foreground,
    );
    ui.describe(id, rect, &Description::new(AccessRole::Button, button.name()));
    ui.tooltip(&response, button.name());
    response.clicked
}

/// A change of a file of the page as its operations are given it: its path, the two versions, its lines on each side
/// and their text.
fn diff_change(state: &DiffPageState<'_>, (file, text, change): (usize, &DiffText, usize)) -> DiffChange {
    let (left, right) = state.versions;
    let path = state.files.get(file).map_or_default(|paths| paths.right.clone());
    let lines = text.changes.get(change).cloned().unwrap_or(model::diff::LineChange {
        left: 0..0,
        right: 0..0,
    });
    DiffChange {
        path,
        left: side_of(left),
        right: side_of(right),
        left_text: lines.left.clone().map(|line| text.left_line(line).to_owned()).collect(),
        right_text: lines
            .right
            .clone()
            .map(|line| text.right_line(line).to_owned())
            .collect(),
        left_lines: lines.left,
        right_lines: lines.right,
    }
}

/// The version of a side as the operations name it.
fn side_of(version: &DiffVersion) -> DiffSide {
    match version {
        DiffVersion::Head => DiffSide::Head,
        DiffVersion::Index => DiffSide::Index,
        DiffVersion::WorkingTree => DiffSide::WorkingTree,
        DiffVersion::Commit { commit, .. } => DiffSide::Commit(commit.clone()),
        // The base of a file in conflict offers no button (`offered`): it is never the side of a change.
        DiffVersion::Empty | DiffVersion::Base | DiffVersion::Resolved(_) => DiffSide::Empty,
    }
}

/// The icon of **Revert this change** in the gutter of a Diff page.
#[cfg(test)]
pub(crate) fn revert_change_icon() -> Icon {
    GutterButton::Revert.icon()
}
