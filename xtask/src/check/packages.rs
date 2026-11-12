//! Rules R1.1, R1.2, R1.5 and R1.6: every library a manifest names is in the allowlist, for that crate; no build
//! scripts and no procedural macros of the workspace's own.

use super::Workspace;
use crate::problem::Problem;
use crate::workspace::Origin;

/// The file that lists the libraries the crates of the workspace may name in their manifests.
pub const ALLOWLIST: &str = "xtask/allowed-packages.txt";

/// A library the allowlist names, with the crates that may use it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allowed {
    /// Its name on crates.io.
    pub name: String,
    /// The crates of the workspace that may name it in their manifest.
    pub users: Vec<String>,
}

/// The libraries of the allowlist: one name per line, followed by the crates that may use it; `#` for comments.
pub fn parse_allowlist(text: &str) -> Vec<Allowed> {
    text.lines()
        .map(|line| line.split('#').next().unwrap_or_default().trim())
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let name = words.next()?;
            Some(Allowed {
                name: name.to_owned(),
                users: words.map(str::to_owned).collect(),
            })
        })
        .collect()
}

/// The problems of the manifests: a library not listed, a library used by a crate the allowlist does not name, a
/// dependency on a folder outside the workspace, a build script or a procedural macro.
pub fn check(workspace: &Workspace) -> Vec<Problem> {
    let allowed = parse_allowlist(workspace.text(ALLOWLIST).unwrap_or_default());
    let mut found = Vec::new();
    let member_folders: Vec<&str> = workspace.members.iter().map(|member| member.folder.as_str()).collect();
    for member in &workspace.members {
        let manifest = format!("{}/Cargo.toml", member.folder);
        if member.builds_specially {
            let message = String::from("The crate has a build script or is a procedural macro. Neither is allowed.");
            found.push(Problem::in_file("R1.2", &manifest, message));
        }
        for dependency in &member.dependencies {
            let message = match &dependency.origin {
                Origin::Folder(folder) if member_folders.contains(&folder.as_str()) => continue,
                Origin::Folder(folder) => format!("{} is in {folder}, outside the workspace.", dependency.package),
                Origin::Outside if may_use(&allowed, &dependency.package, &member.name) => continue,
                Origin::Outside if is_listed(&allowed, &dependency.package) => format!(
                    "{} may not depend on {}: {ALLOWLIST} does not name it among the crates that use it. \
                     Add it there, with the reason in the commit.",
                    member.name, dependency.package
                ),
                Origin::Outside => format!(
                    "{} is not in {ALLOWLIST}. Add it there with the crates that use it, and say why in the commit.",
                    dependency.package
                ),
            };
            found.push(Problem::at_line("R1.1", &manifest, dependency.line, message));
        }
    }
    found
}

fn is_listed(allowed: &[Allowed], name: &str) -> bool {
    allowed.iter().any(|library| library.name == name)
}

/// Whether the allowlist names the crate among those that may depend on the library.
pub fn may_use(allowed: &[Allowed], name: &str, user: &str) -> bool {
    allowed
        .iter()
        .any(|library| library.name == name && library.users.iter().any(|listed| listed == user))
}

#[cfg(test)]
mod tests {
    use super::{Allowed, may_use, parse_allowlist};

    #[test]
    fn reads_the_allowlist() {
        let allowed = parse_allowlist("# The TLS of D13.\nrustls tls\n\nsyntect views render # highlighting\n");
        assert_eq!(
            allowed,
            vec![
                Allowed {
                    name: String::from("rustls"),
                    users: vec![String::from("tls")],
                },
                Allowed {
                    name: String::from("syntect"),
                    users: vec![String::from("views"), String::from("render")],
                },
            ]
        );
    }

    #[test]
    fn lets_a_library_be_used_only_by_the_crates_it_names() {
        let allowed = parse_allowlist("accesskit ui platform\nwinit platform\n");
        assert!(may_use(&allowed, "accesskit", "ui"));
        assert!(may_use(&allowed, "accesskit", "platform"));
        assert!(!may_use(&allowed, "winit", "ui"));
        assert!(!may_use(&allowed, "serde", "model"));
    }
}
