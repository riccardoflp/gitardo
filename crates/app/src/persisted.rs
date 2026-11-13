//! What is kept between runs, in one file per user (`state.json` in Gitardo's configuration folder): whether the
//! program started before, the steps of the Welcome page done, the recent workspaces with the repository shown last in
//! each, the size of the window and the width of the branch rail; the sizes of the panes of each workspace and the
//! folds of each repository ([`timeline`]); the favorites and the branches shown of each repository ([`rail`]).
//! folds of each repository ([`timeline`]); how the files are laid out in the inspector, in the pages of commits and in
//! the Changes page ([`layouts`]); the message of the commit box of each repository ([`changes`]); the values given last
//! for the inputs of the format of branch names in each workspace ([`inputs`]).
//!
//! The file is JSON, written whole each time. A file that cannot be read gives what a first start has.

use std::path::{Path, PathBuf};

use json::{Member, Number, Value};

pub mod changes;
pub mod inputs;
pub mod layouts;
pub mod rail;
pub mod timeline;

pub use changes::RepositoryMessage;
pub use layouts::FileLayouts;
pub use rail::{RepositoryFavorites, RepositoryShown};
pub use timeline::{RepositoryFolds, WorkspacePanes};

/// The name of the file, in Gitardo's configuration folder.
pub const FILE_NAME: &str = "state.json";
/// How many recent workspaces are kept.
pub const MOST_RECENT: usize = 20;
/// How many steps the Welcome page has.
const STEPS: usize = 5;

/// What is kept between runs.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Persisted {
    /// Whether the program started before: the Welcome page shows by itself only the first time.
    pub has_started: bool,
    /// Which steps of the Welcome page are done, in their order.
    pub welcome_steps: [bool; STEPS],
    /// The recent workspaces, the latest first.
    pub recent: Vec<RecentWorkspace>,
    /// The size of the inside of the window, in logical pixels, when it was last closed.
    pub window_size: Option<(f32, f32)>,
    /// The width of the branch rail, in logical pixels, once it was changed.
    pub rail_width: Option<f32>,
    /// The sizes of the panes of the workspaces opened last.
    pub panes: Vec<WorkspacePanes>,
    /// The folds of the repositories shown last.
    pub folds: Vec<RepositoryFolds>,
    /// The favorites of the repositories whose favorites changed last.
    pub favorites: Vec<RepositoryFavorites>,
    /// The branches shown of the repositories narrowed last.
    pub branches_shown: Vec<RepositoryShown>,
    /// Whether the funnel of the search box is pressed: a search of the whole history narrows the timeline to what
    /// it finds (`search.list-mode`).
    pub search_as_list: bool,
    /// How the files are laid out in the inspector, in the pages of commits and in the Changes page.
    pub file_layouts: FileLayouts,
    /// The messages of the commit boxes of the repositories whose message changed last (`changes.pane`).
    pub commit_messages: Vec<RepositoryMessage>,
    /// Whether the Merge pages show the base in a fourth column (`merge-view.show-base`).
    pub merge_shows_base: bool,
    /// The values given last for the inputs of `branches.format`, by workspace (`branch.name-from-format`).
    pub branch_inputs: Vec<inputs::WorkspaceInputs>,
}

/// A workspace opened before.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RecentWorkspace {
    /// Its folders, in their order.
    pub folders: Vec<PathBuf>,
    /// The repository shown in it last, by its folder.
    pub repository: Option<PathBuf>,
}

impl Persisted {
    /// What a file holds; what a first start has when the text is not what this writes.
    #[must_use]
    pub fn from_text(text: &str) -> Persisted {
        let Ok(document) = json::parse(text) else {
            return Persisted::default();
        };
        let mut welcome_steps = [false; STEPS];
        let steps = document
            .get("welcomeSteps")
            .and_then(Value::as_array)
            .unwrap_or_default();
        for (done, step) in welcome_steps.iter_mut().zip(steps) {
            *done = step.as_bool().unwrap_or(false);
        }
        Persisted {
            has_started: document.get("hasStarted").and_then(Value::as_bool).unwrap_or(false),
            welcome_steps,
            recent: document
                .get("recent")
                .and_then(Value::as_array)
                .unwrap_or_default()
                .iter()
                .filter_map(recent_workspace)
                .take(MOST_RECENT)
                .collect(),
            window_size: document.get("windowSize").and_then(size),
            rail_width: document.get("railWidth").and_then(number),
            panes: list(&document, "panes", timeline::workspace_panes),
            folds: list(&document, "folds", timeline::repository_folds),
            favorites: list(&document, "favorites", rail::repository_favorites),
            branches_shown: list(&document, "branchesShown", rail::repository_shown),
            search_as_list: document.get("searchAsList").and_then(Value::as_bool).unwrap_or(false),
            file_layouts: document
                .get("fileLayouts")
                .map(layouts::file_layouts)
                .unwrap_or_default(),
            commit_messages: list(&document, "commitMessages", changes::repository_message),
            merge_shows_base: document.get("mergeShowsBase").and_then(Value::as_bool).unwrap_or(false),
            branch_inputs: list(&document, "branchInputs", inputs::workspace_inputs),
        }
    }

    /// The text of the file.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut members = vec![
            member("hasStarted", Value::Bool(self.has_started)),
            member(
                "welcomeSteps",
                Value::Array(self.welcome_steps.iter().map(|done| Value::Bool(*done)).collect()),
            ),
            member("recent", Value::Array(self.recent.iter().map(recent_value).collect())),
        ];
        if let Some((width, height)) = self.window_size {
            members.push(member(
                "windowSize",
                Value::Array(vec![number_value(width), number_value(height)]),
            ));
        }
        if let Some(width) = self.rail_width {
            members.push(member("railWidth", number_value(width)));
        }
        members.push(member(
            "panes",
            Value::Array(self.panes.iter().map(timeline::panes_value).collect()),
        ));
        members.push(member(
            "folds",
            Value::Array(self.folds.iter().map(timeline::folds_value).collect()),
        ));
        members.push(member(
            "favorites",
            Value::Array(self.favorites.iter().map(rail::favorites_value).collect()),
        ));
        members.push(member(
            "branchesShown",
            Value::Array(self.branches_shown.iter().filter_map(rail::shown_value).collect()),
        ));
        if self.search_as_list {
            members.push(member("searchAsList", Value::Bool(true)));
        }
        if self.file_layouts != FileLayouts::default() {
            members.push(member("fileLayouts", layouts::layouts_value(self.file_layouts)));
        }
        if !self.commit_messages.is_empty() {
            members.push(member(
                "commitMessages",
                Value::Array(self.commit_messages.iter().map(changes::message_value).collect()),
            ));
        }
        if self.merge_shows_base {
            members.push(member("mergeShowsBase", Value::Bool(true)));
        }
        if !self.branch_inputs.is_empty() {
            members.push(member(
                "branchInputs",
                Value::Array(self.branch_inputs.iter().map(inputs::inputs_value).collect()),
            ));
        }
        json::to_pretty_text(&Value::Object(members))
    }

    /// What the file at a path holds; what a first start has when it is not there or cannot be read.
    #[must_use]
    pub fn read(path: &Path) -> Persisted {
        std::fs::read_to_string(path)
            .map(|text| Persisted::from_text(&text))
            .unwrap_or_default()
    }

    /// Puts a workspace first among the recent ones, with the repository shown in it, keeping [`MOST_RECENT`].
    pub fn remember_workspace(&mut self, workspace: RecentWorkspace) {
        self.recent
            .retain(|recent| !same_folders(&recent.folders, &workspace.folders));
        self.recent.insert(0, workspace);
        self.recent.truncate(MOST_RECENT);
    }
}

/// Whether two lists of folders are the same workspace: on Windows the case and the slashes do not count.
#[must_use]
pub fn same_folders(first: &[PathBuf], second: &[PathBuf]) -> bool {
    first.len() == second.len() && first.iter().zip(second).all(|(one, other)| same_folder(one, other))
}

/// Whether two folders are the same: on Windows the case and the slashes do not count.
#[must_use]
pub fn same_folder(first: &Path, second: &Path) -> bool {
    folder_key(first) == folder_key(second)
}

/// A folder as it is compared.
fn folder_key(folder: &Path) -> String {
    let text = folder.to_string_lossy().replace('\\', "/");
    let trimmed = text.trim_end_matches('/');
    if cfg!(windows) {
        trimmed.to_lowercase()
    } else {
        trimmed.to_owned()
    }
}

/// The items of a list of the document that read back, at most [`MOST_RECENT`].
fn list<T>(document: &Value, name: &str, read: fn(&Value) -> Option<T>) -> Vec<T> {
    document
        .get(name)
        .and_then(Value::as_array)
        .unwrap_or_default()
        .iter()
        .filter_map(read)
        .take(MOST_RECENT)
        .collect()
}

/// A member of an object.
fn member(name: &str, value: Value) -> Member {
    Member {
        name: name.to_owned(),
        value,
    }
}

/// A length, as a JSON number.
fn number_value(length: f32) -> Value {
    Number::from_f64(f64::from(length)).map_or(Value::Null, Value::Number)
}

/// A length read back.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the lengths written are those of a window, which an f32 holds"
)]
fn number(value: &Value) -> Option<f32> {
    value.as_number().and_then(Number::as_f64).map(|number| number as f32)
}

/// A size read back: two lengths.
fn size(value: &Value) -> Option<(f32, f32)> {
    match value.as_array()? {
        [width, height] => Some((number(width)?, number(height)?)),
        _ => None,
    }
}

/// A recent workspace, as written.
fn recent_value(workspace: &RecentWorkspace) -> Value {
    let folders = workspace
        .folders
        .iter()
        .map(|folder| Value::String(folder.to_string_lossy().into_owned()))
        .collect();
    let mut members = vec![member("folders", Value::Array(folders))];
    if let Some(repository) = &workspace.repository {
        members.push(member(
            "repository",
            Value::String(repository.to_string_lossy().into_owned()),
        ));
    }
    Value::Object(members)
}

/// A recent workspace read back; none when it has no folder.
fn recent_workspace(value: &Value) -> Option<RecentWorkspace> {
    let folders: Vec<PathBuf> = value
        .get("folders")?
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .map(PathBuf::from)
        .collect();
    (!folders.is_empty()).then(|| RecentWorkspace {
        folders,
        repository: value.get("repository").and_then(Value::as_str).map(PathBuf::from),
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{MOST_RECENT, Persisted, RecentWorkspace, same_folders};

    fn workspace(folder: &str) -> RecentWorkspace {
        RecentWorkspace {
            folders: vec![PathBuf::from(folder)],
            repository: Some(PathBuf::from(format!("{folder}/app"))),
        }
    }

    #[test]
    fn what_is_written_reads_back_the_same() {
        let mut persisted = Persisted {
            has_started: true,
            welcome_steps: [true, false, false, true, false],
            window_size: Some((1280.0, 800.0)),
            rail_width: Some(250.5),
            file_layouts: super::FileLayouts {
                inspector: views::details::FileLayout::Tree,
                page: views::details::FileLayout::List,
                changes: views::details::FileLayout::Tree,
            },
            ..Persisted::default()
        };
        persisted.remember_workspace(workspace("/work"));
        persisted.remember_workspace(workspace("/other"));
        assert_eq!(Persisted::from_text(&persisted.to_text()), persisted);
        assert_eq!(persisted.recent[0].folders, [PathBuf::from("/other")]);
    }

    #[test]
    fn a_text_that_is_not_the_file_gives_a_first_start() {
        assert_eq!(Persisted::from_text("not json"), Persisted::default());
        assert_eq!(Persisted::from_text("[1, 2]"), Persisted::default());
    }

    #[test]
    fn a_workspace_opened_again_moves_first_and_the_list_keeps_its_length() {
        let mut persisted = Persisted::default();
        for number in 0..30 {
            persisted.remember_workspace(workspace(&format!("/work{number}")));
        }
        persisted.remember_workspace(workspace("/work25"));
        assert_eq!(persisted.recent.len(), MOST_RECENT);
        assert_eq!(persisted.recent[0].folders, [PathBuf::from("/work25")]);
        assert_eq!(
            persisted
                .recent
                .iter()
                .filter(|recent| recent.folders[0] == std::path::Path::new("/work25"))
                .count(),
            1
        );
    }

    #[test]
    fn folders_are_compared_as_the_system_does() {
        let first = [PathBuf::from("C:\\Work\\App")];
        let second = [PathBuf::from("c:/work/app/")];
        assert_eq!(same_folders(&first, &second), cfg!(windows));
        assert!(same_folders(&first, &first));
    }
}
