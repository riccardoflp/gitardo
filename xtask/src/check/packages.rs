//! Rules R1.1, R1.2 and R1.5: nothing from outside the workspace but the TLS library, used by `crates/tls` only, at
//! the versions of the allowlist; no build scripts and no procedural macros.

use std::collections::BTreeMap;
use std::path::Path;

use super::Workspace;
use crate::error::Error;
use crate::problem::Problem;
use crate::toml::parse;
use crate::workspace::Origin;

/// The file that lists the packages from outside the workspace that may be in `Cargo.lock`.
pub const ALLOWLIST: &str = "xtask/allowed-packages.txt";

/// The one crate that may depend on them.
const TLS_FOLDER: &str = "crates/tls";

/// A package the allowlist names, at the one version allowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allowed {
    /// Its name on crates.io.
    pub name: String,
    /// Its exact version.
    pub version: String,
}

/// A package of `Cargo.lock`.
#[derive(Debug, Default)]
struct LockedPackage {
    name: String,
    version: String,
    /// Whether the lock gives it a source: a registry or a git repository, never a folder of the workspace.
    comes_from_outside: bool,
    line: usize,
}

/// The packages of the allowlist: one `name version` per line, `#` for comments.
pub fn parse_allowlist(text: &str) -> Vec<Allowed> {
    text.lines()
        .map(|line| line.split('#').next().unwrap_or_default().trim())
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let (name, version) = (words.next()?, words.next()?);
            Some(Allowed {
                name: name.to_owned(),
                version: version.to_owned(),
            })
        })
        .collect()
}

/// The problems of the packages and of the manifests.
pub fn check(root: &Path, workspace: &Workspace) -> Result<Vec<Problem>, Error> {
    let allowed = parse_allowlist(workspace.text(ALLOWLIST).unwrap_or_default());
    let mut found = Vec::new();
    if let Some(lock) = workspace.text("Cargo.lock") {
        let member_names: Vec<&str> = workspace.members.iter().map(|member| member.name.as_str()).collect();
        found.extend(
            lock_problems(lock, &member_names, &allowed).map_err(|malformed| Error::Toml {
                path: root.join("Cargo.lock"),
                line: malformed.line,
                message: malformed.message,
            })?,
        );
    }
    found.extend(manifest_problems(workspace, &allowed));
    Ok(found)
}

/// The packages of `Cargo.lock` that are neither members of the workspace nor in the allowlist at their version.
pub fn lock_problems(
    lock: &str,
    member_names: &[&str],
    allowed: &[Allowed],
) -> Result<Vec<Problem>, crate::toml::Malformed> {
    let mut packages: BTreeMap<usize, LockedPackage> = BTreeMap::new();
    for entry in parse(lock)?.into_iter().filter(|entry| entry.table == ["package"]) {
        let package = packages.entry(entry.group).or_insert_with(|| LockedPackage {
            line: entry.line,
            ..LockedPackage::default()
        });
        let text = entry.value.as_text().unwrap_or_default().to_owned();
        match entry.key.first().map(String::as_str) {
            Some("name") => package.name = text,
            Some("version") => package.version = text,
            Some("source") => package.comes_from_outside = true,
            _ => {} // The checksum and the dependencies of a package say nothing about where it comes from.
        }
    }
    let mut found = Vec::new();
    for package in packages.into_values() {
        let is_member = !package.comes_from_outside && member_names.contains(&package.name.as_str());
        let is_allowed = allowed
            .iter()
            .any(|listed| listed.name == package.name && listed.version == package.version);
        if !is_member && !is_allowed {
            let message = format!(
                "The package {} {} comes from outside the workspace and is not in {ALLOWLIST}. \
                 Remove the dependency that brings it.",
                package.name, package.version
            );
            found.push(Problem::at_line("R1.1", "Cargo.lock", package.line, message));
        }
    }
    Ok(found)
}

fn manifest_problems(workspace: &Workspace, allowed: &[Allowed]) -> Vec<Problem> {
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
                Origin::Outside if member.folder == TLS_FOLDER && is_listed(allowed, &dependency.package) => continue,
                Origin::Outside if is_listed(allowed, &dependency.package) => {
                    let message = format!("Only {TLS_FOLDER} may depend on {}.", dependency.package);
                    found.push(Problem::at_line("R1.5", &manifest, dependency.line, message));
                    continue;
                }
                Origin::Outside => format!("{} is not a crate of the workspace.", dependency.package),
            };
            found.push(Problem::at_line("R1.1", &manifest, dependency.line, message));
        }
    }
    found
}

fn is_listed(allowed: &[Allowed], name: &str) -> bool {
    allowed.iter().any(|package| package.name == name)
}

#[cfg(test)]
mod tests {
    use super::{Allowed, lock_problems, parse_allowlist};

    const LOCK: &str = "version = 4\n\n[[package]]\nname = \"json\"\nversion = \"0.1.0\"\n\n\
        [[package]]\nname = \"ring\"\nversion = \"0.17.14\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n\
        checksum = \"abc\"\ndependencies = [\n \"cc\",\n]\n";

    #[test]
    fn reads_the_allowlist() {
        let allowed = parse_allowlist("# The TLS of D13.\nrustls 0.23.31\n\nring 0.17.14 # cryptography\n");
        assert_eq!(
            allowed,
            vec![
                Allowed {
                    name: String::from("rustls"),
                    version: String::from("0.23.31")
                },
                Allowed {
                    name: String::from("ring"),
                    version: String::from("0.17.14")
                },
            ]
        );
    }

    #[test]
    fn refuses_a_package_from_outside_the_workspace() {
        let found = lock_problems(LOCK, &["json"], &[]).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].rule, "R1.1");
        assert_eq!(found[0].place, "Cargo.lock:8");
        assert!(found[0].message.contains("ring 0.17.14"));
    }

    #[test]
    fn accepts_a_package_of_the_allowlist_at_its_version_only() {
        let at_version = parse_allowlist("ring 0.17.14\n");
        assert_eq!(lock_problems(LOCK, &["json"], &at_version).unwrap(), Vec::new());
        let other_version = parse_allowlist("ring 0.17.13\n");
        assert_eq!(lock_problems(LOCK, &["json"], &other_version).unwrap().len(), 1);
    }

    #[test]
    fn refuses_a_package_that_has_the_name_of_a_member_but_comes_from_outside() {
        let lock = "[[package]]\nname = \"json\"\nversion = \"1.0.0\"\nsource = \"registry+x\"\n";
        assert_eq!(lock_problems(lock, &["json"], &[]).unwrap().len(), 1);
    }
}
