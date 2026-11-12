//! `cargo xtask check`: the rules of `docs/03-code-rules.md` that a program can check, then rustfmt, clippy and the
//! tests.

mod catalog;
mod edges;
mod packages;
mod purity;
mod shape;
mod tools;
mod unsafe_code;

use std::io::Write;
use std::path::Path;

use crate::error::Error;
use crate::files::{SourceFile, read_text, workspace_files};
use crate::problem::Problem;
use crate::workspace::{Member, members};

/// What the checks look at, read once.
pub struct Workspace {
    /// The members of the workspace.
    pub members: Vec<Member>,
    /// Every file of the workspace, with its text when it is text the project writes.
    pub files: Vec<(SourceFile, Option<String>)>,
}

impl Workspace {
    /// Reads the members and the files of the workspace at `root`.
    pub fn read(root: &Path) -> Result<Self, Error> {
        let members = members(root)?;
        let mut files = Vec::new();
        for file in workspace_files(root)? {
            let text = if crate::files::is_text(&file) {
                Some(read_text(&file.path)?)
            } else {
                None
            };
            files.push((file, text));
        }
        Ok(Workspace { members, files })
    }

    /// The Rust files under a folder of the workspace, with their text.
    pub fn rust_files<'a>(&'a self, folder: &'a str) -> impl Iterator<Item = (&'a SourceFile, &'a str)> + 'a {
        self.files.iter().filter_map(move |(file, text)| {
            let is_under =
                file.relative.starts_with(folder) && file.relative.as_bytes().get(folder.len()) == Some(&b'/');
            match text {
                Some(text) if is_under && crate::files::is_rust(file) => Some((file, text.as_str())),
                _ => None,
            }
        })
    }

    /// The text of a file, by its path from the root.
    pub fn text(&self, relative: &str) -> Option<&str> {
        self.files
            .iter()
            .find(|(file, _)| file.relative == relative)
            .and_then(|(_, text)| text.as_deref())
    }
}

/// Every rule a program can check, without running the tools of the toolchain.
pub fn problems(workspace: &Workspace) -> Vec<Problem> {
    let mut found = packages::check(workspace);
    found.extend(edges::check(workspace));
    found.extend(catalog::check(workspace));
    found.extend(purity::check(workspace));
    found.extend(unsafe_code::check(workspace));
    found.extend(shape::check(workspace));
    found.sort();
    found
}

/// Runs every check, says what it found, and returns whether the workspace passed.
pub fn run(root: &Path) -> Result<bool, Error> {
    let workspace = Workspace::read(root)?;
    let found = problems(&workspace);
    let mut output = std::io::stdout().lock();
    for problem in &found {
        writeln!(output, "{problem}")?;
    }
    if found.is_empty() {
        writeln!(output, "The rules of the workspace hold.")?;
    } else {
        writeln!(output, "{} problems with the rules of the workspace.", found.len())?;
    }
    drop(output);
    let tools_passed = tools::run(root)?;
    Ok(found.is_empty() && tools_passed)
}
