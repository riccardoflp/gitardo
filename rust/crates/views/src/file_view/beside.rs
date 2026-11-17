//! A file opened beside the tree of the Files page (`file-view.beside-files`): the tree on the left, as wide as it was
//! dragged, and the File page of the file on its right; in the narrow layout the File page alone.

use ui::{Axis, Rect, Split, Splitter, Ui};

use super::{FilePageState, page_view};
use crate::files::FilesPageState;
use crate::graph::{GraphIntent, Pane, RAIL_LEAST, RAIL_MOST, RAIL_START, STEP};
use crate::intent::Intent;
use crate::line::INSPECTOR_OVER_BELOW;

/// The Files page with a file opened beside its tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BesideFilesState<'a> {
    /// The tree, with the file shown marked.
    pub tree: FilesPageState<'a>,
    /// The File page of the file shown.
    pub file: FilePageState<'a>,
    /// The width of the tree as dragged, in whole logical pixels; none where it starts, as wide as the branch rail
    /// starts.
    pub tree_width: Option<u16>,
}

/// The Files page with a file opened, over the rectangle its caller clipped it to: the tree on the left and the File
/// page on its right, with the line between them that is dragged to resize the tree; in the narrow layout, the File
/// page alone.
///
/// Feature: file-view.beside-files
pub fn beside_files_view(state: &BesideFilesState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    if ui.window().width < INSPECTOR_OVER_BELOW {
        return page_view(&state.file, ui);
    }
    let line = ui.theme().sizes.border;
    let most = rect.width * RAIL_MOST;
    let width = state
        .tree_width
        .map_or(RAIL_START, f32::from)
        .clamp(RAIL_LEAST, most.max(RAIL_LEAST))
        .min(rect.width);
    let tree = Rect::new(rect.x, rect.y, width, rect.height);
    let file = Rect::new(
        tree.right() + line,
        rect.y,
        (rect.width - width - line).max(0.0),
        rect.height,
    );
    let mut intents = ui.clip(tree, |ui| crate::files::page_view(&state.tree, ui));
    ui.separator(Rect::new(tree.right(), rect.y, line, rect.height), Axis::Vertical);
    intents.extend(ui.clip(file, |ui| page_view(&state.file, ui)));
    let splitter = Splitter {
        split: Split::Columns,
        default_size: RAIL_START,
        least_first: RAIL_LEAST,
        least_second: (rect.width - line - most).max(0.0),
        size: Some(width),
        step: STEP,
    };
    let response = ui.splitter_across("files tree line", rect, &splitter, line);
    if response.is_reset {
        intents.push(Intent::Graph(GraphIntent::SetPane(Pane::FilesTree, None)));
    } else if response.is_changed {
        intents.push(Intent::Graph(GraphIntent::SetPane(
            Pane::FilesTree,
            Some(response.size),
        )));
    }
    intents
}
