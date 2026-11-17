//! The comparison chooser (`compare.chooser`): one dialog that chooses both sides of a comparison, **From** and **To**
//! side by side with the arrows that swap them, the list of what the side being chosen can be under one field that
//! filters it, the comparisons opened before at its foot, what `compare.summary` says once both are chosen, and
//! **Compare**.

use model::CommitId;
use model::compare::{CompareSide, RecentComparison, Side, SideInput, follow_sides};
use ui::layout::Area;
use ui::{Dialog, DialogButton, DialogPlace, DialogRole, Label, Rect, TextKind, Tone, Ui};

use super::CompareIntent;
use crate::icons;
use crate::intent::Intent;
use crate::text::compare as words;

mod parts;

use parts::{boxes, field_view, list_view, recent_line};

/// How wide the chooser is, when the window has room.
const WIDTH: f32 = 720.0;

/// The box of the chooser a row chosen fills.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChooserBox {
    /// **From**, the first side.
    From,
    /// **To**, the side compared with it.
    To,
}

impl ChooserBox {
    /// The other box.
    #[must_use]
    pub fn other(self) -> ChooserBox {
        match self {
            ChooserBox::From => ChooserBox::To,
            ChooserBox::To => ChooserBox::From,
        }
    }
}

/// What a side offered by the chooser is, which says its heading and its icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChooserGroup {
    /// What is checked out, or the working tree.
    CheckedOut,
    /// A local branch.
    Branches,
    /// A remote branch.
    RemoteBranches,
    /// A tag.
    Tags,
    /// One of the last commits of the branch checked out.
    RecentCommits,
    /// A revision typed that git knows.
    Typed,
}

/// A side the chooser offers or holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChooserSide {
    /// What it is.
    pub group: ChooserGroup,
    /// The side, as the comparison takes it.
    pub side: CompareSide,
    /// The subject of its commit, shown for a recent commit and matched by what is typed.
    pub subject: String,
    /// When its commit was made, in seconds since 1970, when it is known.
    pub date: Option<i64>,
}

/// What git said of the text typed in the chooser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypedCommit<'a> {
    /// Nothing typed, or git not answered yet.
    Unknown,
    /// The commit it stands for.
    Commit(&'a CommitId),
    /// Git knows no commit by that name.
    NoCommit,
}

/// What the chooser shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChooserState<'a> {
    /// Its title: what it was opened for.
    pub title: &'a str,
    /// What **From** holds.
    pub from: Option<&'a ChooserSide>,
    /// What **To** holds.
    pub to: Option<&'a ChooserSide>,
    /// The box being chosen.
    pub choosing: ChooserBox,
    /// What is typed in the field.
    pub typed: &'a str,
    /// What git says of it.
    pub typed_commit: TypedCommit<'a>,
    /// Every side it offers, in the order of their headings.
    pub rows: &'a [ChooserSide],
    /// The row lit, among those offered now.
    pub lit: usize,
    /// A number that changes each time the row lit or the box chosen changes: the list brings the row lit into sight
    /// and the field takes the keyboard back once for each.
    pub serial: u64,
    /// The comparisons opened before, the latest first.
    pub recent: &'a [RecentComparison],
    /// What `compare.summary` says of the two sides, once both are chosen and read.
    pub summary: Option<&'a str>,
    /// When the repository was read, in seconds since 1970, which how long ago each commit was made counts from.
    pub now: i64,
}

impl ChooserState<'_> {
    /// What the box being chosen holds.
    fn held(&self, which: ChooserBox) -> Option<&ChooserSide> {
        match which {
            ChooserBox::From => self.from,
            ChooserBox::To => self.to,
        }
    }

    /// Whether both boxes hold a side, so that **Compare** can open the comparison.
    #[must_use]
    pub fn can_compare(&self) -> bool {
        self.from.is_some() && self.to.is_some()
    }
}

/// What is asked of the chooser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChooserIntent {
    /// A click on a box, or `Tab`: that box is chosen next.
    Focus(ChooserBox),
    /// The arrows between the boxes: **From** and **To** exchanged.
    Swap,
    /// Text typed in the field.
    Type(String),
    /// Another row lit, by its place among those offered.
    Light(usize),
    /// A row chosen: it fills the box being chosen.
    Choose(Box<ChooserSide>),
    /// **Compare**: the comparison of the two opened.
    Compare,
    /// A comparison opened before, by its place in the list, opened as it was.
    OpenRecent(usize),
    /// `Escape` or **Cancel**: nothing opened.
    Close,
}

/// The sides offered for the box being chosen, in the order shown: **Use `<typed>`** first when what is typed names
/// no row but git knows it, then the rows whose name, hash or subject has what is typed, without what the other box
/// holds.
#[must_use]
pub fn offered(state: &ChooserState<'_>) -> Vec<ChooserSide> {
    let typed = state.typed.trim();
    let lowered = typed.to_lowercase();
    let other = state.held(state.choosing.other()).map(|held| &held.side);
    let matches = |row: &&ChooserSide| {
        let hash = match &row.side.side {
            Side::Commit(commit) => commit.as_str(),
            Side::WorkingTree => "",
        };
        lowered.is_empty()
            || row.side.name.to_lowercase().contains(&lowered)
            || hash.starts_with(&lowered)
            || row.subject.to_lowercase().contains(&lowered)
    };
    let mut offered: Vec<ChooserSide> = state
        .rows
        .iter()
        .filter(|row| other != Some(&row.side))
        .filter(matches)
        .cloned()
        .collect();
    if let TypedCommit::Commit(commit) = state.typed_commit
        && !typed.is_empty()
        && !state.rows.iter().any(|row| row.side.name == typed)
    {
        offered.insert(0, typed_side(typed, commit));
    }
    offered
}

/// The side of a revision typed that git knows: named by the first seven characters of a hash, by what was typed
/// otherwise, as `compare.choose` names it.
fn typed_side(typed: &str, commit: &CommitId) -> ChooserSide {
    let is_hash = commit.as_str().starts_with(&typed.to_lowercase());
    let name = if is_hash { commit.short() } else { typed };
    ChooserSide {
        group: ChooserGroup::Typed,
        side: follow_sides(&SideInput {
            name,
            chosen: &Side::Commit(commit.clone()),
            is_followed: None,
            resolved: Some(commit),
        }),
        subject: String::new(),
        date: None,
    }
}

/// A row of the list: a heading, or a side offered by its place among them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ListRow {
    /// The heading of a group.
    Heading(ChooserGroup),
    /// A side offered.
    Side(usize),
}

/// The rows of the list: the sides offered, with the heading of each group before its first.
fn list_rows(offered: &[ChooserSide]) -> Vec<ListRow> {
    let mut rows = Vec::new();
    let mut last = None;
    for (index, side) in offered.iter().enumerate() {
        if last != Some(side.group) && side.group != ChooserGroup::Typed {
            rows.push(ListRow::Heading(side.group));
        }
        last = Some(side.group);
        rows.push(ListRow::Side(index));
    }
    rows
}

/// The words of the heading of a group.
fn heading(group: ChooserGroup) -> &'static str {
    match group {
        ChooserGroup::CheckedOut => words::CHECKED_OUT,
        ChooserGroup::Branches => words::BRANCHES,
        ChooserGroup::RemoteBranches => words::REMOTE_BRANCHES,
        ChooserGroup::Tags => words::TAGS,
        ChooserGroup::RecentCommits | ChooserGroup::Typed => words::RECENT_COMMITS,
    }
}

/// The icon of a side: what it is, the files for the working tree.
fn icon(side: &ChooserSide) -> ui::Icon {
    if side.side.is_working_tree() {
        return icons::EDITED;
    }
    match side.group {
        ChooserGroup::CheckedOut | ChooserGroup::Branches => icons::BRANCH,
        ChooserGroup::RemoteBranches => icons::REMOTE,
        ChooserGroup::Tags => icons::TAG,
        ChooserGroup::RecentCommits | ChooserGroup::Typed => icons::COMMIT,
    }
}

/// The comparison chooser, over the window at its top: **From** and **To** with the arrows between them, the field
/// that filters the list, the list of what the box being chosen can be, the comparisons opened before, what is said of
/// the two once both are chosen, **Cancel** and **Compare**. Choosing a row fills the box being chosen; `Enter` chooses
/// the row lit, or opens the comparison once both boxes hold a side and nothing is typed; `Escape` opens nothing.
///
/// Feature: compare.chooser
pub fn chooser_view(state: &ChooserState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let theme = *ui.theme();
    let title = ui.line_height(ui.text_style(TextKind::Heading));
    let spacing = theme.sizes.spacing;
    let chrome = 3.0 * spacing.huge + title + spacing.large + theme.sizes.control_height;
    let buttons = [
        DialogButton {
            label: words::CANCEL,
            role: DialogRole::Cancel,
        },
        DialogButton {
            label: words::COMPARE,
            role: if state.can_compare() {
                DialogRole::Primary
            } else {
                DialogRole::Unavailable
            },
        },
    ];
    let dialog = Dialog {
        title: state.title,
        body_height: (ui.window().height * 2.0 / 3.0 - chrome).max(theme.sizes.row_height * 6.0),
        buttons: &buttons,
    };
    let offered = offered(state);
    let mut intents = Vec::new();
    let response = ui.dialog_at(
        "comparison chooser",
        (&dialog, DialogPlace::Top { width: WIDTH }),
        |ui, body| {
            intents = body_view(state, &offered, ui, body.rect);
        },
    );
    if response.is_cancelled {
        intents.push(chooser(ChooserIntent::Close));
    } else if response.pressed == Some(1) && state.can_compare() {
        intents.push(chooser(ChooserIntent::Compare));
    }
    intents
}

/// What is asked of the chooser, as an intent of the window.
fn chooser(intent: ChooserIntent) -> Intent {
    Intent::Compare(CompareIntent::Chooser(intent))
}

/// The body of the chooser, from its top: the boxes, the field, the list, the comparisons opened before and what is
/// said of the two.
fn body_view(state: &ChooserState<'_>, offered: &[ChooserSide], ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut intents = Vec::new();
    let mut area = Area::with_gap(rect, spacing.medium);
    intents.extend(boxes(
        state,
        ui,
        area.top(theme.sizes.control_height + 2.0 * spacing.small),
    ));
    let field = area.top(theme.sizes.control_height);
    let said = area.bottom(theme.sizes.row_height);
    if let Some(summary) = state.summary {
        let _ = ui.label(said, &Label::new(summary).tone(Tone::Muted));
    }
    if !state.recent.is_empty() {
        intents.extend(recent_line(state, ui, area.bottom(theme.sizes.control_height)));
    }
    let list = area.rest();
    intents.extend(field_view(state, offered, ui, field));
    intents.extend(list_view(state, offered, ui, list));
    intents
}
