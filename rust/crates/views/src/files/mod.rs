//! The Files page: the tree of the working tree, its folders folded until they are opened, where a click opens a file
//! in its File page and the context menu of a file or a folder shows its history (`file-history.file`,
//! `file-history.folder`) or compares it with a branch, a tag or a commit (`compare.path-with`). The top row is the
//! repository itself. Each file that differs from the last commit is marked as the Changes page marks it, and each
//! folder holding one has a dot (`file-view.files-changes`).

mod marks;

use std::collections::BTreeSet;

use model::changes::{FolderMark, MarkedFile, TreeMark, TreeMarks};
use model::contract::FeatureId;
use model::details::TreeNode;
use model::state::ChangeKind;
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, FileIconStyle, FileOrFolder, Label, List, MenuEntry, Rect, SelectionMode, Sense, Tone, Ui};

use crate::icons;
use crate::intent::Intent;
use crate::text::changes as changes_words;
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
    /// How the files differ from the last commit, and the dots of the folders holding them.
    pub marks: &'a TreeMarks,
    /// The file shown beside the tree, by its path, which the tree marks (`file-view.beside-files`).
    pub marked: Option<&'a str>,
    /// The files hidden from git, which have an eye struck through at the end of their row (`changes.skipped-list`).
    pub skipped: &'a [model::state::SkippedFile],
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
    /// Open a file beside the tree, by its path (`file-view.beside-files`).
    OpenFile(String),
    /// **Files** in the trail: the tree alone again, without the file opened beside it.
    ShowTree,
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
        /// The dot of the most important state of the files under it, when one differs from the last commit.
        mark: Option<FolderMark>,
    },
    /// A file.
    File {
        /// Its name.
        name: &'a str,
        /// Its path.
        path: &'a str,
        /// How deep it is under the top row.
        depth: usize,
        /// How it differs from the last commit, when it does.
        mark: Option<&'a MarkedFile>,
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
    /// **Search here** (`file-search.open`).
    SearchHere,
    /// **Ignore…** (`changes.ignore`).
    Ignore,
    /// **Stop tracking, keep on disk…** (`changes.stop-tracking`).
    StopTracking,
    /// **Hide its changes from git…** (`changes.skip`).
    Skip,
    /// **Show its changes to git again** (`changes.unskip`).
    Unskip,
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
        tree_rows(nodes, (state.opened, state.marks, 1), &mut rows);
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
        let response = ui.list("files tree", inner, &list, |ui, row| {
            if let Some(files_row) = rows.get(row.index) {
                intents.extend(row_view(ui, row.rect, (state, files_row)));
            }
        });
        // `Enter` on a file opens it, as a click does.
        if let Some(FilesRow::File { path, .. }) = response.activated.and_then(|index| rows.get(index)) {
            intents.push(files_intent(FilesPageIntent::OpenFile((*path).to_owned())));
        }
    });
    intents
}

/// The rows of some nodes of the tree at a depth, without what the folded folders hold.
fn tree_rows<'a>(
    nodes: &'a [TreeNode],
    (opened, marks, depth): (&BTreeSet<String>, &'a TreeMarks, usize),
    rows: &mut Vec<FilesRow<'a>>,
) {
    for node in nodes {
        match node {
            TreeNode::Directory { name, path, children } => {
                let is_open = opened.contains(path);
                rows.push(FilesRow::Folder {
                    name,
                    path,
                    depth,
                    is_open,
                    mark: marks.folders.get(path).copied(),
                });
                if is_open {
                    tree_rows(children, (opened, marks, depth + 1), rows);
                }
            }
            TreeNode::File { name, path, .. } => rows.push(FilesRow::File {
                name,
                path: path.as_str(),
                depth,
                mark: marks.files.get(path.as_str()),
            }),
        }
    }
}

/// A row: its arrow and the icon of its file or folder and its name, lit under the pointer, and how it differs from
/// the last commit; a click opens a file or opens or folds a folder, and its context menu offers what can be done with
/// it.
fn row_view(ui: &mut Ui<'_>, rect: Rect, (state, row): (&FilesPageState<'_>, &FilesRow<'_>)) -> Vec<Intent> {
    let theme = *ui.theme();
    let (name, path, depth) = match *row {
        FilesRow::Top => (state.name, "", 0),
        FilesRow::Folder { name, path, depth, .. } | FilesRow::File { name, path, depth, .. } => (name, path, depth),
    };
    let response = ui.interact(ui.id(("files row", path)), rect, Sense::CLICK | Sense::HOVER);
    let is_marked = matches!(row, FilesRow::File { .. }) && state.marked == Some(path);
    if is_marked {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_selection);
    } else if response.contains_pointer {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    let indent = crate::number::float_of_count(depth) * (theme.sizes.icon + theme.sizes.spacing.small);
    let inner = inset(rect, Insets::symmetric(theme.sizes.spacing.medium, 0.0));
    let mut parts = Area::with_gap(
        Rect::new(inner.x + indent, inner.y, (inner.width - indent).max(0.0), inner.height),
        theme.sizes.spacing.small,
    );
    let opacity = match row {
        FilesRow::File { mark, .. } if marks::is_deleted(*mark) => marks::DIMMED,
        _ => u8::MAX,
    };
    let is_hidden = matches!(row, FilesRow::File { .. }) && state.skipped.iter().any(|file| file.path == path);
    if is_hidden {
        let side = theme.sizes.icon;
        let eye = place(parts.right(side), (side, side), (Align::Start, Align::Center));
        ui.icon_in(eye, icons::HIDE, Tone::Muted);
        let eye_response = ui.interact(ui.id(("hidden from git", path)), eye, Sense::HOVER);
        ui.tooltip(&eye_response, changes_words::HIDDEN_TOOLTIP);
    }
    ui.faded(opacity, |ui| {
        row_icons(ui, &mut parts, row);
        match *row {
            FilesRow::File { mark: Some(file), .. } => marks::marked_file(ui, (parts.rest(), &response), (name, file)),
            FilesRow::Folder { mark: Some(mark), .. } => marks::marked_folder(ui, parts.rest(), (name, mark)),
            _ => {
                let _ = ui.label(parts.rest(), &Label::new(name));
            }
        }
    });
    let mut intents = Vec::new();
    if response.clicked {
        match row {
            FilesRow::Folder { path, .. } => intents.push(files_intent(FilesPageIntent::Fold((*path).to_owned()))),
            FilesRow::File { path, .. } => intents.push(files_intent(FilesPageIntent::OpenFile((*path).to_owned()))),
            FilesRow::Top => {}
        }
    }
    intents.extend(row_menu(ui, &response, (row, path, is_hidden)));
    intents
}

/// The start of a row: the arrow of a folder, and the icon of its file or folder (`file-view.file-icons`). The icon of a
/// folder follows its arrow, and that of a file the place of an arrow, so that the names of a folder line up; with the
/// plain icons, a folder has its arrow alone and a file its icon in the place of the arrow, as before them.
fn row_icons(ui: &mut Ui<'_>, parts: &mut Area, row: &FilesRow<'_>) {
    let side = ui.theme().sizes.icon;
    let cell = |parts: &mut Area| place(parts.left(side), (side, side), (Align::Start, Align::Center));
    let is_simple = ui.file_icons() == FileIconStyle::Simple;
    match *row {
        FilesRow::Top => ui.file_icon_in(cell(parts), FileOrFolder::Top, Tone::Muted),
        FilesRow::Folder { name, is_open, .. } => {
            let arrow = if is_open { icons::UNFOLDED } else { icons::FOLDED };
            ui.icon_in(cell(parts), arrow, Tone::Muted);
            if !is_simple {
                let last = name.rsplit('/').next().unwrap_or(name);
                ui.file_icon_in(cell(parts), FileOrFolder::Folder(last), Tone::Muted);
            }
        }
        FilesRow::File { name, .. } => {
            if !is_simple {
                let _ = cell(parts);
            }
            ui.file_icon_in(cell(parts), FileOrFolder::File(name), Tone::Muted);
        }
    }
}

/// The context menu of a row: **Open file**, **Show file history**, **Compare with…** and **Search here** for a file;
/// **Show folder history**, **Compare with…** and **Search here** for a folder; **Compare with…** and **Search here**
/// for the top folder, whose history is all of it. A file or a folder then has **Ignore…** and **Stop tracking, keep
/// on disk…**, the second not on a file git does not track, and neither on a file deleted or in conflict; a file git
/// tracks has **Hide its changes from git…** too, and a file hidden from git **Show its changes to git again** alone.
fn row_menu(
    ui: &mut Ui<'_>,
    response: &ui::Response,
    (row, path, is_hidden): (&FilesRow<'_>, &str, bool),
) -> Option<Intent> {
    let item = |icon, label, choice| MenuEntry::Item(icons::action(icon, label, choice));
    let compare = item(icons::COMPARE, words::COMPARE_WITH, MenuChoice::Compare);
    let search_here = item(
        icons::SEARCH,
        crate::text::file_search::SEARCH_HERE,
        MenuChoice::SearchHere,
    );
    let mut entries: Vec<MenuEntry<'_, MenuChoice>> = match row {
        FilesRow::File { .. } => vec![
            item(icons::OPEN_FILE, words::OPEN_FILE, MenuChoice::Open),
            item(icons::HISTORY, words::SHOW_FILE_HISTORY, MenuChoice::FileHistory),
            MenuEntry::Separator,
            compare,
            search_here,
        ],
        FilesRow::Folder { .. } => vec![
            item(icons::HISTORY, words::SHOW_FOLDER_HISTORY, MenuChoice::FolderHistory),
            MenuEntry::Separator,
            compare,
            search_here,
        ],
        FilesRow::Top => vec![compare, search_here],
    };
    entries.extend(ignore_entries(row, is_hidden));
    let run = |feature: &'static str| files_intent(FilesPageIntent::Run(FeatureId(feature), path.to_owned()));
    match ui.context_menu(("files row menu", path), response, &entries)? {
        MenuChoice::Open => Some(files_intent(FilesPageIntent::OpenFile(path.to_owned()))),
        MenuChoice::FileHistory => Some(run("file-history.file")),
        MenuChoice::FolderHistory => Some(run("file-history.folder")),
        MenuChoice::Compare => Some(run("compare.path-with")),
        MenuChoice::Ignore => Some(run("changes.ignore")),
        MenuChoice::StopTracking => Some(run("changes.stop-tracking")),
        MenuChoice::Skip => Some(run("changes.skip")),
        MenuChoice::Unskip => Some(run("changes.unskip")),
        MenuChoice::SearchHere => {
            let place = |is_file| model::file_search::Place {
                path: path.to_owned(),
                is_file,
            };
            let place = match row {
                FilesRow::File { .. } => Some(place(true)),
                FilesRow::Folder { .. } => Some(place(false)),
                FilesRow::Top => None,
            };
            Some(Intent::FileSearch(crate::file_search::FileSearchIntent::Here(place)))
        }
    }
}

/// **Ignore…** and **Stop tracking, keep on disk…** under a line, for the rows that have them.
fn ignore_entries(row: &FilesRow<'_>, is_hidden: bool) -> Vec<MenuEntry<'static, MenuChoice>> {
    let (mark, is_file) = match row {
        FilesRow::Top => return Vec::new(),
        FilesRow::File { mark, .. } => (mark.map(|file| file.mark), true),
        FilesRow::Folder { .. } => (None, false),
    };
    let item = |icon, label, choice| MenuEntry::Item(icons::action(icon, label, choice));
    if is_hidden {
        return vec![
            MenuEntry::Separator,
            item(icons::SHOW, changes_words::SHOW_TO_GIT, MenuChoice::Unskip),
        ];
    }
    let mut entries = vec![
        MenuEntry::Separator,
        item(icons::HIDE, changes_words::IGNORE, MenuChoice::Ignore),
    ];
    match mark {
        Some(TreeMark::Conflicted | TreeMark::Changed(ChangeKind::Deleted)) => return Vec::new(),
        Some(TreeMark::Untracked) => return entries,
        _ => entries.push(item(
            icons::UNSTAGE,
            changes_words::STOP_TRACKING,
            MenuChoice::StopTracking,
        )),
    }
    if is_file {
        entries.push(item(icons::HIDE, changes_words::HIDE_FROM_GIT, MenuChoice::Skip));
    }
    entries
}

/// The intent of the window for something asked of the page.
fn files_intent(intent: FilesPageIntent) -> Intent {
    Intent::Files(intent)
}
