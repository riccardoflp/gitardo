//! The files of a commit, in the inspector and in the page of a commit: one after the other with their directories, or
//! under their directories, which fold (`details.file-tree`); the first 300 and a link for the rest
//! (`details.show-all-files`).

use std::collections::BTreeSet;

use model::details::TreeNode;
use model::state::{ChangeKind, FileChange};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{Button, ButtonKind, Color, Icon, Label, Point, Rect, TextKind, TextRun, Tone, Ui};

use crate::commit_page::{CommitPageId, CommitPageIntent};
use crate::details::DetailsIntent;
use crate::intent::Intent;
use crate::text::details as words;

/// How many files a list shows before **Show all**.
pub const FILE_CAP: usize = 300;

/// How the files are laid out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FileLayout {
    /// One after the other, each with its directory beside its name.
    #[default]
    List,
    /// Under their directories, which fold.
    Tree,
}

/// Where a list of files is: what its intents are about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FilesOwner {
    /// The inspector.
    Inspector,
    /// The page of a commit.
    Page(CommitPageId),
    /// The Changes page (`changes.pane`).
    Changes,
}

impl FilesOwner {
    /// The intent of the window for something asked of the files.
    #[must_use]
    pub fn intent(self, files: FilesIntent) -> Intent {
        match self {
            FilesOwner::Inspector => Intent::Details(DetailsIntent::Files(files)),
            FilesOwner::Page(page) => Intent::CommitPage(page, CommitPageIntent::Files(files)),
            FilesOwner::Changes => Intent::Changes(crate::changes::ChangesIntent::Files(files)),
        }
    }
}

/// Something asked of a list of files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilesIntent {
    /// **Show all N files…**.
    ShowAll,
    /// Lay the files out as a list or as a tree; the choice is remembered.
    Layout(FileLayout),
    /// Fold or unfold a directory of the tree, by its path.
    Fold(String),
}

/// A list of files that shows the first 300 until **Show all** is clicked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CappedList {
    /// Whose list it is.
    pub owner: FilesOwner,
    /// How many files it has in all.
    pub count: usize,
    /// Whether **Show all** was clicked.
    pub is_all_shown: bool,
}

impl CappedList {
    /// How many files are shown.
    #[must_use]
    pub fn shown(&self) -> usize {
        if self.is_all_shown {
            self.count
        } else {
            self.count.min(FILE_CAP)
        }
    }

    /// Whether some files are not shown.
    #[must_use]
    pub fn is_cut(&self) -> bool {
        self.shown() < self.count
    }
}

/// The files of a commit as a place shows them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FilesState<'a> {
    /// Every file, in the order git lists them.
    pub files: &'a [FileChange],
    /// The tree of the files shown, for the tree layout.
    pub tree: &'a [TreeNode],
    /// How they are laid out.
    pub layout: FileLayout,
    /// The directories folded, by their paths.
    pub folded: &'a BTreeSet<String>,
    /// How many are shown.
    pub capped: CappedList,
}

/// A row of the files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileRow<'a> {
    /// A directory of the tree.
    Directory {
        /// The name shown.
        name: &'a str,
        /// Its path, which names its fold.
        path: &'a str,
        /// How deep it is.
        depth: usize,
        /// Whether it is folded.
        is_folded: bool,
    },
    /// A file, by its place among the files, and how deep it is in the tree.
    File(usize, usize),
}

/// The rows of the files shown, in their layout, without what the folded directories hold.
pub(crate) fn file_rows<'a>(state: &FilesState<'a>) -> Vec<FileRow<'a>> {
    match state.layout {
        FileLayout::List => (0..state.capped.shown()).map(|index| FileRow::File(index, 0)).collect(),
        FileLayout::Tree => {
            let mut rows = Vec::new();
            tree_rows(state.tree, (state.folded, 0), &mut rows);
            rows
        }
    }
}

/// The rows of some nodes of the tree, at a depth.
fn tree_rows<'a>(nodes: &'a [TreeNode], (folded, depth): (&BTreeSet<String>, usize), rows: &mut Vec<FileRow<'a>>) {
    for node in nodes {
        match node {
            TreeNode::Directory { name, path, children } => {
                let is_folded = folded.contains(path);
                rows.push(FileRow::Directory {
                    name,
                    path,
                    depth,
                    is_folded,
                });
                if !is_folded {
                    tree_rows(children, (folded, depth + 1), rows);
                }
            }
            TreeNode::File { index, .. } => rows.push(FileRow::File(*index, depth)),
        }
    }
}

/// The link after a list of more than 300 files that shows them all, drawn in the rectangle its caller clipped it to,
/// only while some are not shown: **Show all 301 files…**.
///
/// Feature: details.show-all-files
pub fn show_all_files(state: &CappedList, ui: &mut Ui<'_>) -> Vec<Intent> {
    if !state.is_cut() {
        return Vec::new();
    }
    let rect = ui.current_clip();
    let said = words::show_all_files(state.count);
    let link = Button::secondary(&said).kind(ButtonKind::Icon);
    let (width, height) = ui.button_size(&link);
    let cell = place(rect, (width.min(rect.width), height), (Align::Start, Align::Center));
    if ui.button(("show all files", state.owner), cell, &link).clicked {
        return vec![state.owner.intent(FilesIntent::ShowAll)];
    }
    Vec::new()
}

/// The two buttons that choose the layout of the files, at the right of a rectangle, and what the one clicked asks.
pub(crate) fn layout_buttons(
    ui: &mut Ui<'_>,
    area: &mut Area,
    (owner, layout): (FilesOwner, FileLayout),
) -> Vec<Intent> {
    let mut intents = Vec::new();
    let side = ui.theme().sizes.control_height;
    for (choice, icon, said) in [
        (FileLayout::Tree, Icon::Folder, words::AS_TREE),
        (FileLayout::List, Icon::Minus, words::AS_LIST),
    ] {
        let kind = if choice == layout {
            ButtonKind::Secondary
        } else {
            ButtonKind::Icon
        };
        let button = Button::icon(icon, said).kind(kind);
        let cell = place(area.right(side), (side, side), (Align::Start, Align::Center));
        if ui.button(("file layout", owner, said), cell, &button).clicked && choice != layout {
            intents.push(owner.intent(FilesIntent::Layout(choice)));
        }
    }
    intents
}

/// A directory of the tree: its arrow, its icon and its name, a button that folds it.
pub(crate) fn directory_row(ui: &mut Ui<'_>, rect: Rect, (owner, row): (FilesOwner, &FileRow<'_>)) -> Vec<Intent> {
    let FileRow::Directory {
        name,
        path,
        depth,
        is_folded,
    } = *row
    else {
        return Vec::new();
    };
    let indent = indent_of(ui, depth);
    let cell = Rect::new(rect.x + indent, rect.y, (rect.width - indent).max(0.0), rect.height);
    let arrow = if is_folded {
        Icon::ChevronRight
    } else {
        Icon::ChevronDown
    };
    let button = Button::secondary(name).kind(ButtonKind::Icon).with_icon(arrow);
    let (width, height) = ui.button_size(&button);
    let placed = place(
        cell,
        (width.min(cell.width), height.min(cell.height)),
        (Align::Start, Align::Center),
    );
    if ui.button(("directory", owner, path), placed, &button).clicked {
        return vec![owner.intent(FilesIntent::Fold(path.to_owned()))];
    }
    Vec::new()
}

/// How far a row of the tree is indented at a depth.
pub(crate) fn indent_of(ui: &Ui<'_>, depth: usize) -> f32 {
    crate::number::float_of_count(depth) * (ui.theme().sizes.icon + ui.theme().sizes.spacing.small)
}

/// The letter of how a file changed, and its color.
pub(crate) fn status(kind: ChangeKind, ui: &Ui<'_>) -> (&'static str, Color) {
    let colors = ui.theme().colors;
    match kind {
        ChangeKind::Added => ("A", colors.added),
        ChangeKind::Modified => ("M", colors.modified),
        ChangeKind::Deleted => ("D", colors.removed),
        ChangeKind::Renamed => ("R", colors.accent),
        ChangeKind::Copied => ("C", colors.accent),
        ChangeKind::TypeChanged => ("T", colors.modified),
    }
}

/// The start of a row of a file: its status and its icon, then its name; gives the rest of the row.
pub(crate) fn file_start(ui: &mut Ui<'_>, rect: Rect, (file, depth): (&FileChange, usize)) -> Rect {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let indent = indent_of(ui, depth);
    let inner = inset(rect, Insets::symmetric(spacing.medium, 0.0));
    let mut parts = Area::with_gap(
        Rect::new(inner.x + indent, inner.y, (inner.width - indent).max(0.0), inner.height),
        spacing.medium,
    );
    let (letter, color) = status(file.kind, ui);
    let style = ui.text_style(TextKind::Hash);
    let cell = parts.left(ui.measure("M", style));
    let line = place(cell, (cell.width, ui.line_height(style)), (Align::Start, Align::Center));
    let run = TextRun {
        text: letter,
        style,
        color,
        origin: Point::new(line.x, line.y),
        max_width: None,
    };
    let _ = ui.text(&run);
    ui.icon_in(parts.left(theme.sizes.icon), Icon::File, Tone::Muted);
    parts.rest()
}

/// The name of a file and, in the list, its directory beside it in the muted tone.
pub(crate) fn file_name(ui: &mut Ui<'_>, rect: Rect, (file, layout): (&FileChange, FileLayout)) {
    let (directory, name) = match file.path.rsplit_once('/') {
        Some((directory, name)) => (directory, name),
        None => ("", file.path.as_str()),
    };
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.small);
    let width = ui.measure(name, ui.text_style(TextKind::Body)).min(rect.width);
    let _ = ui.label(parts.left(width), &Label::new(name));
    if layout == FileLayout::List && !directory.is_empty() {
        let _ = ui.label(
            parts.rest(),
            &Label::new(directory).kind(TextKind::Detail).tone(Tone::Muted),
        );
    }
}
