//! What the timeline keeps between runs: the sizes of the panes of each workspace (`graph.resize-panes`,
//! `graph.resize-graph`, `graph.toggle-references`) and the folds of each repository (`graph.fold-merge`,
//! `graph.fold-all`).

use std::path::PathBuf;

use json::Value;
use views::graph::PaneSizes;

use super::{MOST_RECENT, member, number, number_value, same_folder, same_folders};

/// How many merges folded, and how many unfolded by hand, are kept for a repository: those folded or unfolded last.
pub const MOST_FOLDS: usize = 200;

/// The sizes of the panes of a workspace, as dragged.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct WorkspacePanes {
    /// The folders of the workspace.
    pub folders: Vec<PathBuf>,
    /// The sizes of the branch rail and the inspector.
    pub sizes: PaneSizes,
    /// The width the graph was dragged to.
    pub graph_width: Option<f32>,
    /// Whether the branch rail is put away.
    pub is_rail_hidden: bool,
}

/// The folds of a repository.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RepositoryFolds {
    /// The folder of the repository.
    pub folder: PathBuf,
    /// The merges folded, by their hashes, the last folded first.
    pub folded: Vec<String>,
    /// The merges unfolded by hand while all are folded, the last first.
    pub unfolded: Vec<String>,
    /// Whether every merge is folded, also those loaded later.
    pub is_all_folded: bool,
}

/// Puts an entry first in a list kept for the last ones used, in place of the one it replaces, keeping [`MOST_RECENT`].
pub fn remember<T>(list: &mut Vec<T>, entry: T, is_same: impl Fn(&T) -> bool) {
    list.retain(|kept| !is_same(kept));
    list.insert(0, entry);
    list.truncate(MOST_RECENT);
}

/// The panes of a workspace kept, when they are.
#[must_use]
pub fn panes_of<'a>(list: &'a [WorkspacePanes], folders: &[PathBuf]) -> Option<&'a WorkspacePanes> {
    list.iter().find(|panes| same_folders(&panes.folders, folders))
}

/// The folds of a repository kept, when they are.
#[must_use]
pub fn folds_of<'a>(list: &'a [RepositoryFolds], folder: &std::path::Path) -> Option<&'a RepositoryFolds> {
    list.iter().find(|folds| same_folder(&folds.folder, folder))
}

/// The panes of a workspace, as written.
pub(super) fn panes_value(panes: &WorkspacePanes) -> Value {
    let mut members = vec![member("folders", folders_value(&panes.folders))];
    let lengths = [
        ("rail", panes.sizes.rail),
        ("inspector", panes.sizes.inspector),
        ("inspectorOver", panes.sizes.inspector_over),
        ("graph", panes.graph_width),
    ];
    for (name, length) in lengths {
        if let Some(length) = length {
            members.push(member(name, number_value(length)));
        }
    }
    members.push(member("railHidden", Value::Bool(panes.is_rail_hidden)));
    Value::Object(members)
}

/// The panes of a workspace read back; none without folders.
pub(super) fn workspace_panes(value: &Value) -> Option<WorkspacePanes> {
    let folders = folders(value)?;
    let length = |name: &str| value.get(name).and_then(number);
    Some(WorkspacePanes {
        folders,
        sizes: PaneSizes {
            rail: length("rail"),
            inspector: length("inspector"),
            inspector_over: length("inspectorOver"),
        },
        graph_width: length("graph"),
        is_rail_hidden: value.get("railHidden").and_then(Value::as_bool).unwrap_or(false),
    })
}

/// The folds of a repository, as written.
pub(super) fn folds_value(folds: &RepositoryFolds) -> Value {
    let hashes = |list: &[String]| Value::Array(list.iter().map(|hash| Value::String(hash.clone())).collect());
    Value::Object(vec![
        member("folder", Value::String(folds.folder.to_string_lossy().into_owned())),
        member("folded", hashes(&folds.folded)),
        member("unfolded", hashes(&folds.unfolded)),
        member("allFolded", Value::Bool(folds.is_all_folded)),
    ])
}

/// The folds of a repository read back; none without a folder.
pub(super) fn repository_folds(value: &Value) -> Option<RepositoryFolds> {
    let hashes = |name: &str| -> Vec<String> {
        value
            .get(name)
            .and_then(Value::as_array)
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .take(MOST_FOLDS)
            .collect()
    };
    Some(RepositoryFolds {
        folder: PathBuf::from(value.get("folder")?.as_str()?),
        folded: hashes("folded"),
        unfolded: hashes("unfolded"),
        is_all_folded: value.get("allFolded").and_then(Value::as_bool).unwrap_or(false),
    })
}

/// Folders, as written.
fn folders_value(folders: &[PathBuf]) -> Value {
    Value::Array(
        folders
            .iter()
            .map(|folder| Value::String(folder.to_string_lossy().into_owned()))
            .collect(),
    )
}

/// Folders read back; none when there are none.
fn folders(value: &Value) -> Option<Vec<PathBuf>> {
    let folders: Vec<PathBuf> = value
        .get("folders")?
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .map(PathBuf::from)
        .collect();
    (!folders.is_empty()).then_some(folders)
}
