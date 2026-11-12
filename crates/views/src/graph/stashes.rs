//! A stash in the timeline (`graph.stashes`): a square with no fill on its dashed line, its label `stash@{0}`, its
//! message, and its context menu, which applies, pops, drops it or makes a branch of it, as the Stashes list of the
//! branch rail does once those operations exist (milestone M4).

use model::commit::Commit;
use ui::layout::{Align, place};
use ui::{Badge, BadgeFill, Icon, MenuEntry, MenuItem, Point, TextKind, TextRun, Tone, Ui};

use crate::intent::Intent;
use crate::text::timeline as words;

/// A stash row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StashRow<'a> {
    /// The commit git keeps it as.
    pub commit: &'a Commit,
    /// Its place among the stashes: 0 for `stash@{0}`.
    pub index: usize,
    /// The color of its line.
    pub color: usize,
}

/// An entry of the context menu of a stash.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StashEntry {
    /// **Apply** (`stash.apply`).
    Apply,
    /// **Pop** (`stash.pop`).
    Pop,
    /// **Create branch from stash…** (`stash.branch`).
    CreateBranch,
    /// **Drop** (`stash.drop`), apart.
    Drop,
}

/// The context menu of a stash: nothing of a commit; its entries cannot be chosen until their operations exist.
#[must_use]
pub fn stash_menu() -> Vec<MenuEntry<'static, StashEntry>> {
    let item = |label, entry| MenuEntry::Item(MenuItem::new(label, entry).disabled());
    vec![
        item(words::APPLY, StashEntry::Apply),
        item(words::POP, StashEntry::Pop),
        item(words::CREATE_BRANCH_FROM_STASH, StashEntry::CreateBranch),
        MenuEntry::Separator,
        item(words::DROP, StashEntry::Drop),
    ]
}

/// The label and the message of a stash over the rectangle of its row its caller clipped it to.
///
/// Feature: graph.stashes
pub fn stashes_view(state: &StashRow<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let name = format!("stash@{{{}}}", state.index);
    let badge = Badge {
        text: &name,
        icons: &[Icon::Stash],
        tone: Tone::Lane(state.color),
        fill: BadgeFill::Outline,
    };
    let style = ui.text_style(TextKind::Medium);
    let line = place(rect, (rect.width, ui.line_height(style)), (Align::Start, Align::Center));
    let drawn = ui.badge(line, &badge);
    let x = drawn.right() + theme.sizes.spacing.medium;
    let run = TextRun {
        text: &state.commit.subject,
        style,
        color: theme.colors.muted_foreground,
        origin: Point::new(x, line.y),
        max_width: Some((rect.right() - x).max(0.0)),
    };
    let _ = ui.text(&run);
    Vec::new()
}
