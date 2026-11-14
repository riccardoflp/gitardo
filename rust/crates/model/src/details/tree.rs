//! The files of a commit, of a comparison or of the changes, laid out under their directories.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;

/// The path of a file from the top of the repository, with `/` between names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FilePath(String);

impl FilePath {
    /// The path of a file, as git gives it.
    #[must_use]
    pub fn new(path: impl Into<String>) -> FilePath {
        FilePath(path.into())
    }

    /// The path, as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for FilePath {
    fn from(path: &str) -> FilePath {
        FilePath::new(path)
    }
}

impl fmt::Display for FilePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A row of the tree: a directory with what it holds, or a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeNode {
    /// A directory, or a chain of directories with a single directory in each, shown as one row.
    Directory {
        /// The name shown: `git`, or `src/git` for a chain.
        name: String,
        /// Its path from the top of the repository, which names its fold.
        path: String,
        /// What it holds, directories first.
        children: Vec<TreeNode>,
    },
    /// A file.
    File {
        /// Its name, without its directories.
        name: String,
        /// Its whole path.
        path: FilePath,
        /// Its place in the list the tree was made from.
        index: usize,
    },
}

impl TreeNode {
    /// The name the row shows.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            TreeNode::Directory { name, .. } | TreeNode::File { name, .. } => name,
        }
    }

    /// How many files are under it, itself for a file.
    #[must_use]
    pub fn file_count(&self) -> usize {
        match self {
            TreeNode::Directory { children, .. } => children.iter().map(TreeNode::file_count).sum(),
            TreeNode::File { .. } => 1,
        }
    }
}

/// The files laid out under their directories: in each directory the directories first, then the files, each group by
/// name without regard to case, and names that differ only in case by their exact characters. A directory that holds
/// nothing but one other directory is one row named with both, down the chain; the top of the tree is never joined so.
/// Each file of the input is in the tree once, and each directory once.
///
/// Feature: details.file-tree
#[must_use]
pub fn file_tree(input: &[FilePath]) -> Vec<TreeNode> {
    let mut top = Folder::default();
    for (index, path) in input.iter().enumerate() {
        top.insert(path, index);
    }
    top.into_nodes("")
}

/// A directory while the tree is being made: the directories in it by name, and its files.
#[derive(Default)]
struct Folder {
    folders: BTreeMap<String, Folder>,
    files: Vec<TreeNode>,
}

impl Folder {
    /// Puts a file in the directories its path names, making them.
    fn insert(&mut self, path: &FilePath, index: usize) {
        let mut names: Vec<&str> = path.as_str().split('/').filter(|name| !name.is_empty()).collect();
        let name = names.pop().unwrap_or_default().to_owned();
        let mut folder = self;
        for directory in names {
            folder = folder.folders.entry(directory.to_owned()).or_default();
        }
        folder.files.push(TreeNode::File {
            name,
            path: path.clone(),
            index,
        });
    }

    /// The rows of what this directory holds, at a path: its directories, then its files, each group by name.
    fn into_nodes(self, path: &str) -> Vec<TreeNode> {
        let mut folders: Vec<(String, Folder)> = self.folders.into_iter().collect();
        folders.sort_by(|(first, _), (second, _)| by_name(first, second));
        let mut nodes: Vec<TreeNode> = folders
            .into_iter()
            .map(|(name, folder)| folder.into_directory(name, path))
            .collect();
        let mut files = self.files;
        files.sort_by(|first, second| by_name(first.name(), second.name()));
        nodes.extend(files);
        nodes
    }

    /// The row of this directory, joined with the directories below it while each holds nothing but one directory.
    fn into_directory(self, name: String, parent: &str) -> TreeNode {
        let mut name = name;
        let mut folder = self;
        while folder.files.is_empty() && folder.folders.len() == 1 {
            let Some((inner_name, inner)) = folder.folders.pop_first() else {
                break;
            };
            name = format!("{name}/{inner_name}");
            folder = inner;
        }
        let path = if parent.is_empty() {
            name.clone()
        } else {
            format!("{parent}/{name}")
        };
        TreeNode::Directory {
            children: folder.into_nodes(&path),
            name,
            path,
        }
    }
}

/// The order of names in a directory: without regard to case, then by their exact characters.
fn by_name(first: &str, second: &str) -> Ordering {
    first
        .to_lowercase()
        .cmp(&second.to_lowercase())
        .then_with(|| first.cmp(second))
}

#[cfg(test)]
mod tests;
