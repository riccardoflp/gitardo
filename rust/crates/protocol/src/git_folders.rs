//! The JSON of what version 1.15 adds for milestone M52 (`repo.watch-git`, D93): the engine watches the git folder of
//! each repository opened itself, and looks again every two seconds at those a view shows. `repository/gitFolderChanged`
//! is the engine saying what it saw change there, which the extension takes as a change of its own watchers;
//! `repository/shown` is the extension saying which repositories its views show, answered with those of them, not
//! shown before, whose git folder changed while nothing was said of it. `repository/changed` goes the other way: it
//! is the extension saying what its own watchers saw.

use serde::{Deserialize, Serialize};

/// The names of the methods of the git folders watched.
pub mod methods {
    /// The notification of the engine: it saw the git folder of a repository opened change (version 1.15).
    pub const GIT_FOLDER_CHANGED: &str = "repository/gitFolderChanged";
    /// The request of the extension: the repositories its views show, in place of those said before (version 1.15).
    pub const SHOWN: &str = "repository/shown";
}

/// The path the engine says when it cannot name the files: the git folder itself, when its watcher lost changes or
/// something was found changed that no watcher said.
pub const GIT_FOLDER: &str = ".git";

/// The parameters of `repository/gitFolderChanged`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFolderChangedParams {
    /// The repository, as `repository/open` named it.
    pub repository: String,
    /// What changed, as `repository/changed` writes the paths of a git folder: from the common git folder, with `/`
    /// and a leading `.git/`, such as `.git/refs/heads/main`; `.git` alone when the engine cannot name the files.
    /// Only paths that ask for a read (`repo.watch-changes`), and never none.
    pub paths: Vec<String>,
    /// Whether the change was found by looking at the git folder again, its watcher having said nothing of it.
    #[serde(default)]
    pub found: bool,
}

/// The parameters of `repository/shown`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShownParams {
    /// The repositories a view shows now, each as `repository/open` named it. They take the place of those said
    /// before; one that is not open is left aside.
    #[serde(default)]
    pub repositories: Vec<String>,
}

/// The answer of `repository/shown`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShownResult {
    /// Those of the repositories named that were not shown before and whose git folder changed since the engine last
    /// looked at it or said a change of it: what the extension kept of them is out of date.
    pub changed: Vec<String>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{GitFolderChangedParams, ShownParams, ShownResult};

    #[test]
    fn the_messages_of_the_git_folders_are_written_as_the_protocol_page_says() {
        let changed = GitFolderChangedParams {
            repository: String::from("/work/app"),
            paths: vec![String::from(".git/refs/heads/main")],
            found: false,
        };
        assert_eq!(
            serde_json::to_value(&changed).unwrap(),
            json!({ "repository": "/work/app", "paths": [".git/refs/heads/main"], "found": false })
        );
        let shown: ShownParams = serde_json::from_value(json!({ "repositories": ["/work/app"] })).unwrap();
        assert_eq!(shown.repositories, [String::from("/work/app")]);
        let answer = ShownResult {
            changed: vec![String::from("/work/app")],
        };
        assert_eq!(
            serde_json::to_value(&answer).unwrap(),
            json!({ "changed": ["/work/app"] })
        );
    }
}
