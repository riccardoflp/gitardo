//! The Files page: the tree of the working tree, its folders folded until they are opened, where a click opens a file
//! in its File page and the context menu of a file or a folder shows its history (`file-history.file`,
//! `file-history.folder`) or compares it with a branch, a tag or a commit (`compare.path-with`). The top row is the
//! repository itself.

use std::collections::BTreeSet;

use model::contract::FeatureId;
use model::details::TreeNode;
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Icon, Label, List, MenuEntry, MenuItem, Rect, SelectionMode, Sense, Tone, Ui};

use crate::intent::Intent;
use crate::text::files as words;

/// What the Files page shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FilesPageState<'a> {
    /// The name of the repository, on the top row.
    pub name: &'a str,
    /// The files listed, or why there are none to show.
    pub shown: FilesShown<'a>,
    /// The folders opened, by their paths; the others are folded.
    pub opened: &'a BTreeSet<String>,
}

/// The files of the page, or why there are none to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilesShown<'a> {
    /// They are being listed.
    Reading,
    /// Git could not list them: what it said.
    Failed(&'a str),
    /// The tree of the files listed.
    Listed(&'a [TreeNode]),
}

/// Something asked of the Files page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilesPageIntent {
    /// Open the page, listing the files again.
    Open,
    /// Open or fold a folder, by its path.
    Fold(String),
    /// Open a file in its File page, by its path.
    OpenFile(String),
    /// Run an operation on a file or a folder, by its path from the top of the working tree; empty for the top folder.
    Run(FeatureId, String),
}

/// A row of the tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FilesRow<'a> {
    /// The repository itself.
    Top,
    /// A folder.
    Folder {
        /// The name shown: `src`, or `src/git` for a chain of folders.
        name: &'a str,
        /// Its path.
        path: &'a str,
        /// How deep it is under the top row.
        depth: usize,
        /// Whether it is opened.
        is_open: bool,
    },
    /// A file.
    File {
        /// Its name.
        name: &'a str,
        /// Its path.
        path: &'a str,
        /// How deep it is under the top row.
        depth: usize,
    },
}

/// An entry of the context menu of a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum MenuChoice {
    /// **Open file**.
    Open,
    /// **Show file history**.
    FileHistory,
    /// **Show folder history**.
    FolderHistory,
    /// **Compare with Branch, Tag or Commit…**.
    Compare,
}

/// The Files page over the rectangle its caller clipped it to: the tree of the files of the working tree under the row
/// of the repository, or what it says while they are listed or when they cannot be.
pub fn page_view(state: &FilesPageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let mut intents = Vec::new();
    ui.region("files page", (rect, AccessRole::Main, words::PAGE), |ui| {
        let inner = inset(rect, Insets::all(theme.sizes.spacing.large));
        let nodes = match state.shown {
            FilesShown::Listed(nodes) => nodes,
            FilesShown::Reading | FilesShown::Failed(_) => {
                let said = match state.shown {
                    FilesShown::Failed(error) => words::failed(error),
                    _ => String::from(words::READING),
                };
                let line = Rect::new(inner.x, inner.y, inner.width, theme.sizes.row_height);
                let _ = ui.label(line, &Label::new(&said).tone(Tone::Muted));
                return;
            }
        };
        let mut rows = vec![FilesRow::Top];
        tree_rows(nodes, (state.opened, 1), &mut rows);
        if nodes.is_empty() {
            let line = Rect::new(inner.x, inner.y, inner.width, theme.sizes.row_height);
            let _ = ui.label(line, &Label::new(words::NO_FILES).tone(Tone::Muted));
            return;
        }
        let list = List {
            name: words::TREE,
            row_count: rows.len(),
            row_height: theme.sizes.row_height,
            selection_mode: SelectionMode::Single,
            lead: 0.0,
        };
        let _ = ui.list("files tree", inner, &list, |ui, row| {
            if let Some(files_row) = rows.get(row.index) {
                intents.extend(row_view(ui, row.rect, (state, files_row)));
            }
        });
    });
    intents
}

/// The rows of some nodes of the tree at a depth, without what the folded folders hold.
fn tree_rows<'a>(nodes: &'a [TreeNode], (opened, depth): (&BTreeSet<String>, usize), rows: &mut Vec<FilesRow<'a>>) {
    for node in nodes {
        match node {
            TreeNode::Directory { name, path, children } => {
                let is_open = opened.contains(path);
                rows.push(FilesRow::Folder {
                    name,
                    path,
                    depth,
                    is_open,
                });
                if is_open {
                    tree_rows(children, (opened, depth + 1), rows);
                }
            }
            TreeNode::File { name, path, .. } => rows.push(FilesRow::File {
                name,
                path: path.as_str(),
                depth,
            }),
        }
    }
}

/// A row: its arrow and icon and its name, lit under the pointer; a click opens a file or opens or folds a folder, and
/// its context menu offers what can be done with it.
fn row_view(ui: &mut Ui<'_>, rect: Rect, (state, row): (&FilesPageState<'_>, &FilesRow<'_>)) -> Vec<Intent> {
    let theme = *ui.theme();
    let (name, path, depth, icon) = match *row {
        FilesRow::Top => (state.name, "", 0, Icon::Folder),
        FilesRow::Folder {
            name,
            path,
            depth,
            is_open,
        } => {
            let arrow = if is_open { Icon::ChevronDown } else { Icon::ChevronRight };
            (name, path, depth, arrow)
        }
        FilesRow::File { name, path, depth } => (name, path, depth, Icon::File),
    };
    let response = ui.interact(ui.id(("files row", path)), rect, Sense::CLICK | Sense::HOVER);
    if response.hovered {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    let indent = crate::number::float_of_count(depth) * (theme.sizes.icon + theme.sizes.spacing.small);
    let inner = inset(rect, Insets::symmetric(theme.sizes.spacing.medium, 0.0));
    let mut parts = Area::with_gap(
        Rect::new(inner.x + indent, inner.y, (inner.width - indent).max(0.0), inner.height),
        theme.sizes.spacing.small,
    );
    let icon_cell = place(
        parts.left(theme.sizes.icon),
        (theme.sizes.icon, theme.sizes.icon),
        (Align::Start, Align::Center),
    );
    ui.icon_in(icon_cell, icon, Tone::Muted);
    let _ = ui.label(parts.rest(), &Label::new(name));
    let mut intents = Vec::new();
    if response.clicked {
        match row {
            FilesRow::Folder { path, .. } => intents.push(files_intent(FilesPageIntent::Fold((*path).to_owned()))),
            FilesRow::File { path, .. } => intents.push(files_intent(FilesPageIntent::OpenFile((*path).to_owned()))),
            FilesRow::Top => {}
        }
    }
    intents.extend(row_menu(ui, &response, (row, path)));
    intents
}

/// The context menu of a row: **Open file**, **Show file history** and **Compare with…** for a file; **Show folder
/// history** and **Compare with…** for a folder; **Compare with…** alone for the top folder, whose history is all of it.
fn row_menu(ui: &mut Ui<'_>, response: &ui::Response, (row, path): (&FilesRow<'_>, &str)) -> Option<Intent> {
    let item = |label, choice| MenuEntry::Item(MenuItem::new(label, choice));
    let entries: Vec<MenuEntry<'_, MenuChoice>> = match row {
        FilesRow::File { .. } => vec![
            item(words::OPEN_FILE, MenuChoice::Open),
            item(words::SHOW_FILE_HISTORY, MenuChoice::FileHistory),
            MenuEntry::Separator,
            item(words::COMPARE_WITH, MenuChoice::Compare),
        ],
        FilesRow::Folder { .. } => vec![
            item(words::SHOW_FOLDER_HISTORY, MenuChoice::FolderHistory),
            MenuEntry::Separator,
            item(words::COMPARE_WITH, MenuChoice::Compare),
        ],
        FilesRow::Top => vec![item(words::COMPARE_WITH, MenuChoice::Compare)],
    };
    let run = |feature: &'static str| files_intent(FilesPageIntent::Run(FeatureId(feature), path.to_owned()));
    match ui.context_menu(("files row menu", path), response, &entries)? {
        MenuChoice::Open => Some(files_intent(FilesPageIntent::OpenFile(path.to_owned()))),
        MenuChoice::FileHistory => Some(run("file-history.file")),
        MenuChoice::FolderHistory => Some(run("file-history.folder")),
        MenuChoice::Compare => Some(run("compare.path-with")),
    }
}

/// The intent of the window for something asked of the page.
fn files_intent(intent: FilesPageIntent) -> Intent {
    Intent::Files(intent)
}
