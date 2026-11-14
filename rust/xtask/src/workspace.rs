//! The crates of the workspace, read from the manifests: their names, their folders and what they depend on.

use std::path::{Component, Path, PathBuf};

use crate::error::Error;
use crate::files::read_text;
use crate::toml::{Entry, Value, parse};

/// A crate of the workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// The name of its package.
    pub name: String,
    /// Its folder, from the root of the workspace, with `/`: `crates/json`.
    pub folder: String,
    /// What it depends on, as its manifest says.
    pub dependencies: Vec<Dependency>,
    /// Whether its manifest names a build script, or makes it a procedural macro.
    pub builds_specially: bool,
}

/// One dependency of a crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    /// The name of the package it is on.
    pub package: String,
    /// For what the crate needs it.
    pub use_kind: UseKind,
    /// Where it comes from.
    pub origin: Origin,
    /// The line of the manifest that names it.
    pub line: usize,
}

/// For what a crate needs one of its dependencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseKind {
    /// To build its code.
    Normal,
    /// Only to build its tests.
    Development,
    /// To run its build script.
    Build,
}

/// Where a dependency comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// A folder on the disk, from the root of the workspace, with `/`.
    Folder(String),
    /// Anything else: a registry, a git repository, or the workspace's own table of dependencies.
    Outside,
}

/// The members of the workspace, in the order of the root manifest.
pub fn members(root: &Path) -> Result<Vec<Member>, Error> {
    let root_manifest = root.join("Cargo.toml");
    let entries = read_manifest(&root_manifest)?;
    let folders = entries
        .iter()
        .filter(|entry| entry.table == ["workspace"] && entry.key == ["members"])
        .find_map(|entry| match &entry.value {
            Value::Array(values) => Some(values.iter().filter_map(Value::as_text).map(str::to_owned).collect()),
            Value::Text(_) | Value::Boolean(_) | Value::Bare(_) | Value::Table(_) => None,
        })
        .unwrap_or_else(Vec::new);
    folders.iter().map(|folder| member(root, folder)).collect()
}

fn member(root: &Path, folder: &str) -> Result<Member, Error> {
    let entries = read_manifest(&root.join(folder).join("Cargo.toml"))?;
    let name = entries
        .iter()
        .find(|entry| entry.table == ["package"] && entry.key == ["name"])
        .and_then(|entry| entry.value.as_text())
        .unwrap_or(folder)
        .to_owned();
    let builds_specially = entries.iter().any(|entry| {
        (entry.table == ["package"] && entry.key == ["build"])
            || (entry.table == ["lib"] && entry.key == ["proc-macro"])
    }) || root.join(folder).join("build.rs").exists();
    let dependencies = dependencies(&entries, folder);
    Ok(Member {
        name,
        folder: folder.to_owned(),
        dependencies,
        builds_specially,
    })
}

fn read_manifest(path: &Path) -> Result<Vec<Entry>, Error> {
    let text = read_text(path)?;
    parse(&text).map_err(|malformed| Error::Toml {
        path: path.to_path_buf(),
        line: malformed.line,
        message: malformed.message,
    })
}

/// The dependencies a manifest names, in every form Cargo accepts: `name = "1.0"`, `name = { path = … }`,
/// `name.path = …`, `[dependencies.name]`, and the same under `[target.<condition>.…]`.
fn dependencies(entries: &[Entry], folder: &str) -> Vec<Dependency> {
    let mut found: Vec<Dependency> = Vec::new();
    for entry in entries {
        let Some((use_kind, name, field)) = dependency_of(entry) else {
            continue;
        };
        let origin = origin_of(field.as_deref(), &entry.value, folder);
        match found
            .iter_mut()
            .find(|dependency| dependency.package == name && dependency.use_kind == use_kind)
        {
            Some(known) if known.origin == Origin::Outside => known.origin = origin,
            Some(_) => {}
            None => found.push(Dependency {
                package: name,
                use_kind,
                origin,
                line: entry.line,
            }),
        }
    }
    found
}

/// The kind, the name and the field of the dependency an entry is about, if it is about one.
fn dependency_of(entry: &Entry) -> Option<(UseKind, String, Option<String>)> {
    let use_kind_of = |segment: &str| match segment {
        "dependencies" => Some(UseKind::Normal),
        "dev-dependencies" => Some(UseKind::Development),
        "build-dependencies" => Some(UseKind::Build),
        _ => None,
    };
    if let Some(use_kind) = entry.table.last().and_then(|segment| use_kind_of(segment)) {
        let name = entry.key.first()?.clone();
        return Some((use_kind, name, entry.key.get(1).cloned()));
    }
    let (name, table) = entry.table.split_last()?;
    let use_kind = use_kind_of(table.last()?)?;
    Some((use_kind, name.clone(), entry.key.first().cloned()))
}

fn origin_of(field: Option<&str>, value: &Value, folder: &str) -> Origin {
    let path = match field {
        None => value.member("path").and_then(Value::as_text),
        Some("path") => value.as_text(),
        Some(_) => None,
    };
    match path {
        Some(path) => Origin::Folder(joined(folder, path)),
        None => Origin::Outside,
    }
}

/// `crates/app` and `../json` make `crates/json`.
fn joined(folder: &str, relative: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    for component in PathBuf::from(folder).join(relative).components() {
        match component {
            Component::ParentDir => {
                let _ = parts.pop();
            }
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
        }
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::{Origin, UseKind, dependencies, joined};
    use crate::toml::parse;

    #[test]
    fn reads_every_form_of_dependency() {
        let manifest = "[dependencies]\njson = { path = \"../json\" }\nring = \"0.17\"\nmodel.path = \"../model\"\n\
            [dev-dependencies.testkit]\npath = \"../testkit\"\n\
            [target.'cfg(windows)'.build-dependencies]\ncc = { version = \"1\" }\n";
        let found = dependencies(&parse(manifest).unwrap(), "crates/app");
        let summary: Vec<(&str, UseKind, Origin)> = found
            .iter()
            .map(|dependency| {
                (
                    dependency.package.as_str(),
                    dependency.use_kind,
                    dependency.origin.clone(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            vec![
                ("json", UseKind::Normal, Origin::Folder(String::from("crates/json"))),
                ("ring", UseKind::Normal, Origin::Outside),
                ("model", UseKind::Normal, Origin::Folder(String::from("crates/model"))),
                (
                    "testkit",
                    UseKind::Development,
                    Origin::Folder(String::from("crates/testkit"))
                ),
                ("cc", UseKind::Build, Origin::Outside),
            ]
        );
    }

    #[test]
    fn joins_relative_folders() {
        assert_eq!(joined("crates/app", "../json"), "crates/json");
        assert_eq!(joined("xtask", "./../crates/git"), "crates/git");
    }
}
