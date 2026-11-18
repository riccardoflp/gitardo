//! What the timeline keeps between runs: the sizes of the panes of each workspace (`graph.resize-panes`,
//! `graph.resize-graph`, `graph.toggle-references`, `repo.side-column`) and the folds of each repository (`graph.fold-merge`,
//! `graph.fold-all`).

use std::path::PathBuf;

use json::{Member, Value};
use views::graph::PaneSizes;
use views::panel::PanelTab;

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
    /// The width of the results of the Search page beside the preview (`file-search.preview`).
    pub search_results: Option<f32>,
    /// Whether the side column is put away.
    pub is_rail_hidden: bool,
    /// Whether **Repositories** is folded in the side column (`repo.side-column`).
    pub is_repositories_folded: bool,
    /// Whether **Branches** is folded in the side column.
    pub is_branches_folded: bool,
    /// The repositories open whose worktrees and submodules are folded under their rows in **Repositories**, by their
    /// folders (`repo.group-tabs`).
    pub folded_tabs: Vec<PathBuf>,
    /// The bottom panel (`panel.bottom`).
    pub panel: PanelLook,
}

/// How the bottom panel is: shown or not, its height and its tab (`panel.bottom`).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct PanelLook {
    /// Whether it is shown.
    pub is_shown: bool,
    /// The height it was dragged to; none while it is where it starts.
    pub height: Option<f32>,
    /// The tab it shows.
    pub tab: PanelTab,
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
        ("repositories", panes.sizes.repositories),
        ("inspector", panes.sizes.inspector),
        ("inspectorOver", panes.sizes.inspector_over),
        ("filesTree", panes.sizes.files_tree),
        ("changesColumn", panes.sizes.changes_column),
        ("searchResults", panes.search_results),
        ("graph", panes.graph_width),
    ];
    for (name, length) in lengths {
        if let Some(length) = length {
            members.push(member(name, number_value(length)));
        }
    }
    members.push(member("railHidden", Value::Bool(panes.is_rail_hidden)));
    for (name, is_folded) in [
        ("repositoriesFolded", panes.is_repositories_folded),
        ("branchesFolded", panes.is_branches_folded),
    ] {
        if is_folded {
            members.push(member(name, Value::Bool(true)));
        }
    }
    if !panes.folded_tabs.is_empty() {
        members.push(member("foldedTabs", folders_value(&panes.folded_tabs)));
    }
    members.extend(panel_members(panes.panel));
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
            repositories: length("repositories"),
            inspector: length("inspector"),
            inspector_over: length("inspectorOver"),
            files_tree: length("filesTree"),
            changes_column: length("changesColumn"),
        },
        graph_width: length("graph"),
        search_results: length("searchResults"),
        is_rail_hidden: value.get("railHidden").and_then(Value::as_bool).unwrap_or(false),
        is_repositories_folded: value
            .get("repositoriesFolded")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        is_branches_folded: value.get("branchesFolded").and_then(Value::as_bool).unwrap_or(false),
        folded_tabs: listed_folders(value, "foldedTabs").unwrap_or_default(),
        panel: panel_look(value),
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
    listed_folders(value, "folders")
}

/// The folders of a member read back; none when there are none.
fn listed_folders(value: &Value, name: &str) -> Option<Vec<PathBuf>> {
    let folders: Vec<PathBuf> = value
        .get(name)?
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .map(PathBuf::from)
        .collect();
    (!folders.is_empty()).then_some(folders)
}

/// The names the tabs of the bottom panel are written with.
const PANEL_TABS: [(PanelTab, &str); 3] = [
    (PanelTab::Terminal, "terminal"),
    (PanelTab::Activity, "activity"),
    (PanelTab::CommandLog, "commandLog"),
];

/// The bottom panel of a workspace, as written: nothing for a panel as it starts, hidden at its terminal.
fn panel_members(panel: PanelLook) -> Vec<Member> {
    let mut members = Vec::new();
    if panel.is_shown {
        members.push(member("panelShown", Value::Bool(true)));
    }
    if let Some(height) = panel.height {
        members.push(member("panelHeight", number_value(height)));
    }
    if let Some((_, name)) = PANEL_TABS
        .iter()
        .find(|(tab, _)| *tab == panel.tab && *tab != PanelTab::Terminal)
    {
        members.push(member("panelTab", Value::String((*name).to_owned())));
    }
    members
}

/// The bottom panel of a workspace read back.
fn panel_look(value: &Value) -> PanelLook {
    let tab = value.get("panelTab").and_then(Value::as_str);
    PanelLook {
        is_shown: value.get("panelShown").and_then(Value::as_bool).unwrap_or(false),
        height: value.get("panelHeight").and_then(number),
        tab: PANEL_TABS
            .iter()
            .find(|(_, name)| Some(*name) == tab)
            .map_or(PanelTab::Terminal, |(tab, _)| *tab),
    }
}
