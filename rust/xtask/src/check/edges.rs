//! Rule R1.3: a crate depends only on the crates the picture of `docs/desktop/02-architecture.md` lets it use.

use super::Workspace;
use crate::problem::Problem;
use crate::workspace::{Origin, UseKind};

/// The crates each crate may use directly, as the picture of "Who may use whom" draws them, with `app` using every
/// crate below it (decision D15) and `engine` using `model` (decision D14).
const ALLOWED: [(&str, &[&str]); 20] = [
    (
        "app",
        &[
            "views", "engine", "features", "settings", "json", "http", "ui", "editor", "render", "platform", "system",
            "git", "model", "text", "tls",
        ],
    ),
    ("views", &["ui", "model", "features"]),
    ("ui", &["render", "editor"]),
    ("editor", &["render"]),
    ("render", &["platform"]),
    ("platform", &["system"]),
    ("system", &[]),
    ("features", &["model", "text", "json", "git"]),
    ("engine", &["git", "model", "http", "json", "system"]),
    ("settings", &["json", "model"]),
    ("git", &["model", "text"]),
    ("model", &[]),
    ("text", &[]),
    ("json", &[]),
    ("http", &["tls"]),
    ("tls", &[]),
    ("protocol", &["model", "git", "engine", "text"]),
    (
        "server",
        &["protocol", "engine", "features", "settings", "git", "model", "json", "text", "http"],
    ),
    ("testkit", &[]),
    ("xtask", &[]),
];

/// The crate every crate may use for its tests only.
const TEST_ONLY: &str = "testkit";

/// One crate depending on another.
#[derive(Debug, Clone, Copy)]
pub struct Edge<'a> {
    /// The crate whose manifest names the other.
    pub dependent: &'a str,
    /// The crate it names.
    pub dependency: &'a str,
    /// For what it needs it.
    pub use_kind: UseKind,
}

/// Whether the picture has an edge; `None` when the dependent crate is not in the picture at all.
pub fn is_allowed(edge: Edge<'_>) -> Option<bool> {
    let (_, allowed) = ALLOWED.iter().find(|(crate_name, _)| *crate_name == edge.dependent)?;
    let for_tests =
        edge.use_kind == UseKind::Development && edge.dependency == TEST_ONLY && edge.dependent != TEST_ONLY;
    Some(for_tests || allowed.contains(&edge.dependency))
}

/// The dependencies between crates of the workspace that the picture does not have.
pub fn check(workspace: &Workspace) -> Vec<Problem> {
    let mut found = Vec::new();
    for member in &workspace.members {
        let manifest = format!("{}/Cargo.toml", member.folder);
        for dependency in &member.dependencies {
            let Origin::Folder(folder) = &dependency.origin else {
                continue;
            };
            let Some(used) = workspace.members.iter().find(|other| other.folder == *folder) else {
                continue;
            };
            let edge = Edge {
                dependent: &member.name,
                dependency: &used.name,
                use_kind: dependency.use_kind,
            };
            let message = match is_allowed(edge) {
                Some(true) => continue,
                Some(false) => format!(
                    "{} may not use {}: the picture of docs/desktop/02-architecture.md has no such edge. Change the \
                     picture first, in its own commit.",
                    member.name, used.name
                ),
                None => format!(
                    "{} is not in the picture of docs/desktop/02-architecture.md.",
                    member.name
                ),
            };
            found.push(Problem::at_line("R1.3", &manifest, dependency.line, message));
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::{Edge, is_allowed};
    use crate::workspace::UseKind;

    #[test]
    fn follows_the_picture() {
        let cases = [
            ("views", "ui", UseKind::Normal, Some(true)),
            ("ui", "views", UseKind::Normal, Some(false)),
            ("model", "git", UseKind::Normal, Some(false)),
            ("features", "git", UseKind::Normal, Some(true)),
            ("json", "testkit", UseKind::Development, Some(true)),
            ("json", "testkit", UseKind::Normal, Some(false)),
            ("testkit", "testkit", UseKind::Development, Some(false)),
            ("render", "platform", UseKind::Build, Some(true)),
            ("engine", "system", UseKind::Normal, Some(true)),
            ("engine", "platform", UseKind::Normal, Some(false)),
            ("server", "platform", UseKind::Normal, Some(false)),
            ("app", "server", UseKind::Normal, Some(false)),
            ("somewhere", "json", UseKind::Normal, None),
        ];
        for (dependent, dependency, use_kind, expected) in cases {
            let edge = Edge {
                dependent,
                dependency,
                use_kind,
            };
            assert_eq!(is_allowed(edge), expected, "{dependent} → {dependency}");
        }
    }
}
