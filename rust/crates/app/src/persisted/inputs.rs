//! The values given last for the inputs of `branches.format` in each workspace, by its folders, under `branchInputs`
//! (`branch.name-from-format`).

use std::collections::BTreeMap;
use std::path::PathBuf;

use json::Value;

use super::{MOST_RECENT, member, same_folders};

/// The values given last for the inputs of the format in a workspace.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct WorkspaceInputs {
    /// The folders of the workspace, in their order.
    pub folders: Vec<PathBuf>,
    /// The value given last for each input, by its id.
    pub values: BTreeMap<String, String>,
}

/// The values given last in a workspace; none when none was given.
#[must_use]
pub fn inputs_of(list: &[WorkspaceInputs], folders: &[PathBuf]) -> BTreeMap<String, String> {
    list.iter()
        .find(|kept| same_folders(&kept.folders, folders))
        .map(|kept| kept.values.clone())
        .unwrap_or_default()
}

/// Keeps values given in a workspace over those given before for the same inputs, the workspace first in the list; at
/// most [`MOST_RECENT`] workspaces are kept.
pub fn remember_inputs(list: &mut Vec<WorkspaceInputs>, folders: &[PathBuf], given: &BTreeMap<String, String>) {
    let mut values = inputs_of(list, folders);
    values.extend(given.iter().map(|(id, value)| (id.clone(), value.clone())));
    list.retain(|kept| !same_folders(&kept.folders, folders));
    list.insert(
        0,
        WorkspaceInputs {
            folders: folders.to_vec(),
            values,
        },
    );
    list.truncate(MOST_RECENT);
}

/// The values of a workspace, as written.
pub(super) fn inputs_value(kept: &WorkspaceInputs) -> Value {
    let folders = kept
        .folders
        .iter()
        .map(|folder| Value::String(folder.to_string_lossy().into_owned()))
        .collect();
    let values = kept
        .values
        .iter()
        .map(|(id, value)| member(id, Value::String(value.clone())))
        .collect();
    Value::Object(vec![
        member("folders", Value::Array(folders)),
        member("values", Value::Object(values)),
    ])
}

/// The values of a workspace read back; none without a folder.
pub(super) fn workspace_inputs(value: &Value) -> Option<WorkspaceInputs> {
    let folders: Vec<PathBuf> = value
        .get("folders")?
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .map(PathBuf::from)
        .collect();
    let values = value
        .get("values")
        .and_then(Value::as_object)
        .unwrap_or_default()
        .iter()
        .filter_map(|kept| Some((kept.name.clone(), kept.value.as_str()?.to_owned())))
        .collect();
    (!folders.is_empty()).then_some(WorkspaceInputs { folders, values })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use super::{inputs_of, remember_inputs};
    use crate::persisted::Persisted;

    #[test]
    fn the_values_given_are_kept_by_workspace_and_read_back() {
        let work = vec![PathBuf::from("/work")];
        let mut persisted = Persisted::default();
        let given: BTreeMap<String, String> = [(String::from("ticket"), String::from("12"))].into();
        remember_inputs(&mut persisted.branch_inputs, &work, &given);
        let kind: BTreeMap<String, String> = [(String::from("kind"), String::from("fix"))].into();
        remember_inputs(&mut persisted.branch_inputs, &work, &kind);
        let values = inputs_of(&persisted.branch_inputs, &work);
        assert_eq!(values.get("ticket").map(String::as_str), Some("12"));
        assert_eq!(values.get("kind").map(String::as_str), Some("fix"));
        assert_eq!(
            inputs_of(&persisted.branch_inputs, &[PathBuf::from("/other")]),
            BTreeMap::new()
        );
        assert_eq!(Persisted::from_text(&persisted.to_text()), persisted);
    }
}
