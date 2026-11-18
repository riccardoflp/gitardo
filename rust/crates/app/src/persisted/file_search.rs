//! What the Search page keeps between runs (`file-search.remember`), for each repository by its folder, under
//! `fileSearches`: what is typed, the switches, the mode, the scope and the filters.

use std::path::{Path, PathBuf};

use json::Value;
use model::file_search::{Mode, Place, Scope, Search};

use super::{MOST_RECENT, member, same_folder};

/// The search of a repository, as kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositorySearch {
    /// The folder of the repository.
    pub folder: PathBuf,
    /// What is searched there.
    pub search: Search,
}

/// Keeps the search of a repository, the latest first, for the 20 repositories searched last; a search as a first one
/// is, with nothing typed and nothing changed, is not kept, and takes the place of the one kept before.
///
/// Feature: file-search.remember
pub fn keep_search(list: &mut Vec<RepositorySearch>, folder: &Path, search: &Search) {
    list.retain(|kept| !same_folder(&kept.folder, folder));
    if *search == Search::default() {
        return;
    }
    list.insert(
        0,
        RepositorySearch {
            folder: folder.to_path_buf(),
            search: search.clone(),
        },
    );
    list.truncate(MOST_RECENT);
}

/// The search kept for a repository, when one is.
#[must_use]
pub fn search_of<'a>(list: &'a [RepositorySearch], folder: &Path) -> Option<&'a Search> {
    list.iter()
        .find(|kept| same_folder(&kept.folder, folder))
        .map(|kept| &kept.search)
}

/// The searches kept, as written under `fileSearches`; nothing while none is kept.
pub(super) fn searches_member(list: &[RepositorySearch]) -> Option<json::Member> {
    (!list.is_empty()).then(|| member("fileSearches", Value::Array(list.iter().map(search_value).collect())))
}

/// The search of a repository, as written: only what differs from a first search.
fn search_value(kept: &RepositorySearch) -> Value {
    let search = &kept.search;
    let mut members = vec![member(
        "folder",
        Value::String(kept.folder.to_string_lossy().into_owned()),
    )];
    let texts = [
        ("query", &search.query),
        ("include", &search.include),
        ("exclude", &search.exclude),
    ];
    for (name, text) in texts {
        if !text.is_empty() {
            members.push(member(name, Value::String(text.clone())));
        }
    }
    let switches = [
        ("matchCase", search.match_case),
        ("wholeWord", search.whole_word),
        ("regex", search.regex),
        ("gitignore", search.gitignore),
        ("hidden", search.hidden),
    ];
    members.extend(switches.into_iter().map(|(name, on)| member(name, Value::Bool(on))));
    members.push(member("mode", Value::String(mode_word(search.mode).to_owned())));
    members.push(member("scope", Value::String(scope_word(search.scope).to_owned())));
    if let Some(named) = &search.named {
        members.push(member("named", Value::String(named.path.clone())));
        members.push(member("namedIsFile", Value::Bool(named.is_file)));
    }
    Value::Object(members)
}

/// The search of a repository read back; none without a folder, or in a shape this version does not know.
pub(super) fn repository_search(value: &Value) -> Option<RepositorySearch> {
    let folder = PathBuf::from(value.get("folder")?.as_str()?);
    let text = |name: &str| value.get(name).and_then(Value::as_str).unwrap_or_default().to_owned();
    let switch = |name: &str, first: bool| match value.get(name) {
        None => Some(first),
        Some(found) => found.as_bool(),
    };
    let named = match value.get("named") {
        None => None,
        Some(path) => Some(Place {
            path: path.as_str()?.to_owned(),
            is_file: switch("namedIsFile", false)?,
        }),
    };
    let scope = scope_of(value.get("scope").map_or(Some("repository"), Value::as_str)?)?;
    let search = Search {
        query: text("query"),
        match_case: switch("matchCase", false)?,
        whole_word: switch("wholeWord", false)?,
        regex: switch("regex", false)?,
        mode: mode_of(value.get("mode").map_or(Some("all"), Value::as_str)?)?,
        scope: if scope == Scope::Named && named.is_none() {
            Scope::Repository
        } else {
            scope
        },
        named,
        include: text("include"),
        exclude: text("exclude"),
        gitignore: switch("gitignore", true)?,
        hidden: switch("hidden", false)?,
    };
    Some(RepositorySearch { folder, search })
}

/// The word a mode is kept as.
fn mode_word(mode: Mode) -> &'static str {
    match mode {
        Mode::All => "all",
        Mode::Text => "text",
        Mode::Files => "files",
        Mode::Folders => "folders",
    }
}

/// The mode of a word kept; none for a word this version does not know.
fn mode_of(word: &str) -> Option<Mode> {
    Mode::EVERY.into_iter().find(|mode| mode_word(*mode) == word)
}

/// The word a scope is kept as.
fn scope_word(scope: Scope) -> &'static str {
    match scope {
        Scope::Repository => "repository",
        Scope::Changes => "changes",
        Scope::Named => "named",
    }
}

/// The scope of a word kept; none for a word this version does not know.
fn scope_of(word: &str) -> Option<Scope> {
    [Scope::Repository, Scope::Changes, Scope::Named]
        .into_iter()
        .find(|scope| scope_word(*scope) == word)
}

#[cfg(test)]
#[path = "file_search_tests.rs"]
mod tests;
