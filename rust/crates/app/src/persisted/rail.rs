//! What the branch rail and the branch filter keep between runs, for each repository by its folder: the favorites
//! (`refs.favorite`), under `favorites`, the branches the timeline shows (`filter.remember`), under `branchesShown`, and
//! what the branch rail lists (`refs.listed`), under `branchesListed` in the same shape.

use std::path::{Path, PathBuf};

use json::Value;
use model::filter::{MOST_TICKED, StoredValue};

use super::{member, same_folder};

/// The favorites of a repository.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RepositoryFavorites {
    /// The folder of the repository.
    pub folder: PathBuf,
    /// The full names of the branches, in the order they were added.
    pub references: Vec<String>,
}

/// The branches a repository's timeline shows, or its branch rail lists, as kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryShown {
    /// The folder of the repository.
    pub folder: PathBuf,
    /// The choice, in the words it is kept in.
    pub shown: StoredValue,
}

/// The favorites kept for a repository; none when there are none.
#[must_use]
pub fn favorites_of<'a>(list: &'a [RepositoryFavorites], folder: &Path) -> &'a [String] {
    list.iter()
        .find(|favorites| same_folder(&favorites.folder, folder))
        .map_or_default(|favorites| favorites.references.as_slice())
}

/// The choice kept for a repository, when one is.
#[must_use]
pub fn shown_of<'a>(list: &'a [RepositoryShown], folder: &Path) -> Option<&'a StoredValue> {
    list.iter()
        .find(|shown| same_folder(&shown.folder, folder))
        .map(|shown| &shown.shown)
}

/// The favorites of a repository, as written.
pub(super) fn favorites_value(favorites: &RepositoryFavorites) -> Value {
    Value::Object(vec![
        member("folder", Value::String(favorites.folder.to_string_lossy().into_owned())),
        member("references", strings_value(&favorites.references)),
    ])
}

/// The favorites of a repository read back; none without a folder.
pub(super) fn repository_favorites(value: &Value) -> Option<RepositoryFavorites> {
    Some(RepositoryFavorites {
        folder: PathBuf::from(value.get("folder")?.as_str()?),
        references: strings(value.get("references")),
    })
}

/// The choice of a repository, as written: `preset`, `kinds` or `references` beside its folder; nothing for a shape
/// this version does not know.
pub(super) fn shown_value(shown: &RepositoryShown) -> Option<Value> {
    let (name, value) = match &shown.shown {
        StoredValue::Preset(word) => ("preset", Value::String(word.clone())),
        StoredValue::Kinds(words) => ("kinds", strings_value(words)),
        StoredValue::References(names) => ("references", strings_value(names)),
        StoredValue::Unknown => return None,
    };
    Some(Value::Object(vec![
        member("folder", Value::String(shown.folder.to_string_lossy().into_owned())),
        member(name, value),
    ]))
}

/// What the branch rail lists of each repository, as written under `branchesListed`; nothing while every rail lists
/// everything.
pub(super) fn listed_member(list: &[RepositoryShown]) -> Option<json::Member> {
    (!list.is_empty()).then(|| {
        member(
            "branchesListed",
            Value::Array(list.iter().filter_map(shown_value).collect()),
        )
    })
}

/// The choice of a repository read back; none without a folder, and a shape this version does not know as such.
pub(super) fn repository_shown(value: &Value) -> Option<RepositoryShown> {
    let folder = PathBuf::from(value.get("folder")?.as_str()?);
    let shown = if let Some(word) = value.get("preset").and_then(Value::as_str) {
        StoredValue::Preset(word.to_owned())
    } else if let Some(kinds) = value.get("kinds").filter(|kinds| kinds.as_array().is_some()) {
        StoredValue::Kinds(strings(Some(kinds)))
    } else if let Some(names) = value.get("references").filter(|names| names.as_array().is_some()) {
        StoredValue::References(strings(Some(names)).into_iter().take(MOST_TICKED).collect())
    } else {
        StoredValue::Unknown
    };
    Some(RepositoryShown { folder, shown })
}

/// Strings, as written.
fn strings_value(list: &[String]) -> Value {
    Value::Array(list.iter().map(|text| Value::String(text.clone())).collect())
}

/// The strings of a list read back.
fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}
