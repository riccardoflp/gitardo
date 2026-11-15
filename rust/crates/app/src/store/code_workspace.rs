//! Opening a workspace file of VS Code (`repo.open-code-workspace`): the folders it lists, opened as a folder is.

use std::path::{Path, PathBuf};

use json::Value;

use super::feature::Said;
use super::message::Effect;
use super::state::{Opened, State};

/// Opens the folders of a workspace file of VS Code chosen in the system's dialog: at once in a window with no
/// workspace; otherwise **Replace the workspace** or **Add to the workspace** is asked. A file that names no folders,
/// or that cannot be read, is refused and says why.
///
/// Feature: repo.open-code-workspace
pub fn open_code_workspace(state: &mut State, chosen: Option<(PathBuf, Result<String, String>)>) -> Vec<Effect> {
    let Some((file, text)) = chosen else {
        return Vec::new();
    };
    let folders = text.and_then(|text| code_workspace_folders(&text, &file));
    let folders = match folders {
        Ok(folders) => folders,
        Err(said) => {
            state.operations.said = Some(Said::error(said));
            return Vec::new();
        }
    };
    super::open_folders(
        state,
        Opened {
            folders,
            file: Some(file),
        },
    )
}

/// The folders a workspace file of VS Code lists: the `path` of each entry of `folders`, in their order, a relative
/// path from the folder of the file. Comments and trailing commas are allowed, as VS Code allows them.
///
/// # Errors
///
/// What is wrong with the text, or "This file names no folders." when it lists none.
pub fn code_workspace_folders(text: &str, file: &Path) -> Result<Vec<PathBuf>, String> {
    let document = json::parse_with_comments(text).map_err(|error| error.to_string())?;
    let base = file.parent().unwrap_or_else(|| Path::new(""));
    let folders: Vec<PathBuf> = document
        .get("folders")
        .and_then(Value::as_array)
        .unwrap_or_default()
        .iter()
        .filter_map(|folder| folder.get("path").and_then(Value::as_str))
        .map(|path| normal(&base.join(path)))
        .collect();
    if folders.is_empty() {
        return Err(String::from(views::text::repo::NAMES_NO_FOLDERS));
    }
    Ok(folders)
}

/// A path with its `.` and `..` taken out, as far as they go, without asking the system: `work/../web` is `web`.
fn normal(path: &Path) -> PathBuf {
    let mut parts: Vec<std::path::Component<'_>> = Vec::new();
    for part in path.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir if matches!(parts.last(), Some(std::path::Component::Normal(_))) => {
                let _gone = parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.iter().collect()
}
