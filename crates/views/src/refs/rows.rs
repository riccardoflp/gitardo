//! The rows of the branch rail: which there are, how tall each is, and how each is drawn.

use std::path::Path;

use model::state::{Branch, RemoteBranch, RepositoryState, Worktree};
use ui::layout::{Area, Insets, inset};
use ui::{AccessRole, Description, Icon, Label, LineCap, Rect, Sense, Stroke, TextKind, Theme, Tone, Ui};

use super::ReferencesState;
use crate::intent::{Intent, KeptList};
use crate::text::header;
use crate::text::rail as words;

/// The height of the title of a list.
const TITLE: f32 = 20.0;
/// The height of the rows of the local branches.
const BRANCH_ROW: f32 = 32.0;
/// The height of the rows of the remote branches and of the lists of **Kept aside**.
const QUIET_ROW: f32 = 30.0;
/// The side of the dot of a branch.
const DOT: f32 = 10.0;

/// A row of the rail.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum RailRow<'a> {
    /// The title of a list: **LOCAL**, **ORIGIN**, **KEPT ASIDE**.
    Title(String),
    /// A local branch, with the color of its line in the timeline.
    Branch {
        /// The color of its line, none when its commit is not shown.
        color: Option<usize>,
        /// The branch.
        branch: &'a Branch,
    },
    /// A remote branch.
    Remote(&'a RemoteBranch),
    /// A list of **Kept aside**, with how many it holds and whether it is open.
    Kept {
        /// Which list.
        list: KeptList,
        /// How many it holds.
        count: usize,
        /// Whether it is open.
        is_open: bool,
    },
    /// An entry of an open list of **Kept aside**, by its name.
    Entry(String),
    /// Room between two lists.
    Gap,
}

impl RailRow<'_> {
    /// The height of the row, in logical pixels.
    pub(super) fn height(&self, theme: &Theme) -> f32 {
        match self {
            RailRow::Title(_) => TITLE,
            RailRow::Branch { .. } => BRANCH_ROW,
            RailRow::Remote(_) | RailRow::Kept { .. } => QUIET_ROW,
            RailRow::Entry(_) => theme.sizes.row_height,
            RailRow::Gap => theme.sizes.spacing.large + theme.sizes.spacing.small,
        }
    }
}

/// The rows of the rail of a repository: the local branches, the branches of each remote, and **Kept aside** with the
/// entries of its lists that are open. In the narrow layout there are no titles and no entries.
pub(super) fn rail_rows<'a>(state: &ReferencesState<'a>, is_icons: bool) -> Vec<RailRow<'a>> {
    let repository = state.repository;
    let mut rows = Vec::new();
    let title = |text: &str| (!is_icons).then(|| RailRow::Title(text.to_owned()));
    rows.extend(title(words::LOCAL));
    rows.extend(repository.branches.iter().map(|branch| RailRow::Branch {
        color: state.branch_colors.get(&branch.name).copied(),
        branch,
    }));
    for remote in &repository.remotes {
        rows.push(RailRow::Gap);
        rows.extend(title(&words::remote_title(remote)));
        rows.extend(
            repository
                .remote_branches
                .iter()
                .filter(|branch| &branch.remote == remote)
                .map(RailRow::Remote),
        );
    }
    rows.push(RailRow::Gap);
    rows.extend(title(words::KEPT_ASIDE));
    for list in KeptList::ALL {
        let entries = kept_entries(repository, list);
        let is_open = state.open.contains(&list);
        rows.push(RailRow::Kept {
            list,
            count: entries.len(),
            is_open,
        });
        if is_open && !is_icons {
            rows.extend(entries.into_iter().map(RailRow::Entry));
        }
    }
    rows
}

/// The names of the entries of a list of **Kept aside**: each stash by its message, each worktree by the name of its
/// folder with its branch, `detached` or `missing`, each tag by its name.
fn kept_entries(repository: &RepositoryState, list: KeptList) -> Vec<String> {
    match list {
        KeptList::Stashes => repository.stashes.iter().map(|stash| stash.message.clone()).collect(),
        KeptList::Worktrees => repository.worktrees.iter().map(worktree_name).collect(),
        KeptList::Tags => repository.tags.iter().map(|tag| tag.name.clone()).collect(),
    }
}

/// A worktree as the rail names it: its folder's name, then its branch, `detached` or `missing`.
fn worktree_name(worktree: &Worktree) -> String {
    let folder = worktree
        .folder
        .file_name()
        .map_or_else(
            || worktree.folder.to_string_lossy(),
            |name| Path::new(name).to_string_lossy(),
        )
        .into_owned();
    let stands = if worktree.is_missing {
        words::MISSING
    } else {
        worktree.branch.as_deref().map_or(words::DETACHED, |branch| {
            branch.strip_prefix("refs/heads/").unwrap_or(branch)
        })
    };
    format!("{folder} \u{b7} {stands}")
}

/// The name of a list of **Kept aside**.
pub(super) fn kept_name(list: KeptList) -> &'static str {
    match list {
        KeptList::Stashes => words::STASHES,
        KeptList::Worktrees => words::WORKTREES,
        KeptList::Tags => words::TAGS,
    }
}

/// The title of a list.
pub(super) fn title(ui: &mut Ui<'_>, rect: Rect, text: &str) {
    let padded = inset(rect, Insets::symmetric(ui.theme().sizes.spacing.medium, 0.0));
    let _ = ui.label(padded, &Label::new(text).kind(TextKind::SmallStrong).tone(Tone::Muted));
}

/// A local branch: the dot of its lane, its name, and how far it is from its upstream, `↑1 ↓2`, or `gone`.
pub(super) fn branch(ui: &mut Ui<'_>, rect: Rect, (color, branch): (Option<usize>, &Branch), is_icons: bool) {
    let theme = *ui.theme();
    let color = color.map_or(theme.colors.muted_foreground, |color| theme.lane(color));
    if is_icons {
        ui.fill_circle(rect.center(), DOT * 0.5, color);
        let id = ui.id(("branch dot", &branch.name));
        ui.describe(id, rect, &Description::new(AccessRole::Label, &branch.name));
        return;
    }
    let spacing = theme.sizes.spacing;
    let mut parts = Area::with_gap(
        inset(rect, Insets::symmetric(spacing.medium, 0.0)),
        spacing.medium + 2.0,
    );
    let dot = parts.left(DOT);
    ui.fill_circle(dot.center(), DOT * 0.5, color);
    let stands = branch.upstream.as_ref().map(|upstream| {
        if upstream.is_gone {
            words::GONE.to_owned()
        } else {
            header::upstream_distance(upstream.ahead, upstream.behind)
        }
    });
    if let Some(stands) = &stands {
        let width = ui.measure(stands, ui.text_style(TextKind::Small));
        let _ = ui.label(
            parts.right(width),
            &Label::new(stands).kind(TextKind::Small).tone(Tone::Muted),
        );
    }
    let kind = if branch.is_checked_out {
        TextKind::Strong
    } else {
        TextKind::Medium
    };
    let _ = ui.label(parts.rest(), &Label::new(&branch.name).kind(kind));
}

/// A remote branch: a ring and its name, quieter than the local branches.
pub(super) fn remote(ui: &mut Ui<'_>, rect: Rect, branch: &RemoteBranch, is_icons: bool) {
    let theme = *ui.theme();
    let ring = Stroke {
        width: theme.sizes.graph_line,
        cap: LineCap::Butt,
        color: theme.colors.muted_foreground,
    };
    let radius = (DOT - theme.sizes.graph_line) * 0.5;
    if is_icons {
        ui.stroke_circle(rect.center(), radius, ring);
        let id = ui.id(("remote ring", &branch.name));
        ui.describe(id, rect, &Description::new(AccessRole::Label, &branch.name));
        return;
    }
    let spacing = theme.sizes.spacing;
    let mut parts = Area::with_gap(
        inset(rect, Insets::symmetric(spacing.medium, 0.0)),
        spacing.medium + 2.0,
    );
    let dot = parts.left(DOT);
    ui.stroke_circle(dot.center(), radius, ring);
    let _ = ui.label(parts.rest(), &Label::new(&branch.name).tone(Tone::Muted));
}

/// A list of **Kept aside**: its name and how many it holds, which a click opens or closes.
pub(super) fn kept(ui: &mut Ui<'_>, rect: Rect, (list, count, is_open): (KeptList, usize, bool)) -> Option<Intent> {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let id = ui.id(("kept", list));
    let response = ui.interact(id, rect, Sense::CLICK | Sense::FOCUS);
    if response.hovered {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    if response.shows_focus {
        ui.focus_ring(rect, theme.sizes.radius.medium);
    }
    let name = kept_name(list);
    let count_text = count.to_string();
    let mut parts = Area::with_gap(inset(rect, Insets::symmetric(spacing.medium, 0.0)), spacing.small);
    let chevron = if is_open { Icon::ChevronDown } else { Icon::ChevronRight };
    ui.icon_in(parts.left(theme.sizes.icon), chevron, Tone::Muted);
    let count_width = ui.measure(&count_text, ui.text_style(TextKind::Body));
    let _ = ui.label(parts.right(count_width), &Label::new(&count_text).tone(Tone::Muted));
    let _ = ui.label(parts.rest(), &Label::new(name));
    let description = Description::new(AccessRole::Button, name)
        .value(&count_text)
        .expanded(is_open);
    ui.describe(id, rect, &description);
    response.clicked.then_some(Intent::ToggleKept(list))
}

/// An entry of an open list of **Kept aside**, under its name.
pub(super) fn entry(ui: &mut Ui<'_>, rect: Rect, text: &str) {
    let theme = *ui.theme();
    let indent = theme.sizes.spacing.medium + theme.sizes.icon + theme.sizes.spacing.small;
    let shown = Rect::new(rect.x + indent, rect.y, (rect.width - indent).max(0.0), rect.height);
    let _ = ui.label(shown, &Label::new(text).kind(TextKind::Detail).tone(Tone::Muted));
}
