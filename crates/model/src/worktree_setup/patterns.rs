//! What the link and copy patterns take from the checkout: the folders a new worktree gets as links, and the files and
//! folders it gets as copies. Every path is from the top of the checkout, with `/` and none at its end.

use std::collections::BTreeSet;

/// What git says of the checkout, for the folders to link.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LinkInput {
    /// The files git ignores (`git ls-files --others --ignored --exclude-standard`).
    pub ignored_files: Vec<String>,
    /// The folders git ignores whole (the same with `--directory`).
    pub ignored_whole: Vec<String>,
    /// The files git does not track that the patterns match.
    pub matched_files: Vec<String>,
    /// The folders the patterns name as folders.
    pub named_folders: Vec<String>,
    /// Of the folders that hold a file both ignored and matched ([`folders_holding_matches`]), those git ignores.
    pub ignored_folders: Vec<String>,
}

/// What git says of the checkout and of the new worktree's commit, for the paths to copy.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CopyInput {
    /// The files git ignores.
    pub ignored_files: Vec<String>,
    /// The folders git ignores whole, empty ones left out (`--directory --no-empty-directory`).
    pub ignored_whole: Vec<String>,
    /// The files git does not track that the patterns match.
    pub matched_files: Vec<String>,
    /// The folders that are linked, whose content is not copied.
    pub linked_folders: Vec<String>,
    /// The files the new worktree has from its commit.
    pub commit_files: Vec<String>,
}

/// What the patterns of the page take from the checkout, as a whole and each on its own.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SetupPreview {
    /// The folders all the link patterns link.
    pub linked: Vec<String>,
    /// The files and folders all the copy patterns copy, with the links of all the patterns in place.
    pub copied: Vec<String>,
    /// How many folders each link pattern links on its own.
    pub link_patterns: Vec<PatternTake>,
    /// How many files or folders each copy pattern copies on its own, with the links of all the patterns in place.
    pub copy_patterns: Vec<PatternTake>,
}

/// How much one pattern takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternTake {
    /// The pattern, as written.
    pub pattern: String,
    /// How many folders, or files and folders, it takes.
    pub count: usize,
}

/// The folders that hold a file git ignores and the patterns match, each once, by path: the folders a link could be.
#[must_use]
pub fn folders_holding_matches(ignored_files: &[String], matched_files: &[String]) -> Vec<String> {
    let matched: BTreeSet<&str> = matched_files.iter().map(String::as_str).collect();
    let mut folders = BTreeSet::new();
    for file in ignored_files.iter().filter(|file| matched.contains(file.as_str())) {
        let mut folder = file.as_str();
        while let Some((parent, _)) = folder.rsplit_once('/') {
            let _ = folders.insert(parent.to_owned());
            folder = parent;
        }
    }
    folders.into_iter().collect()
}

/// The folders a new worktree gets as links: those that hold a file git ignores and the patterns match, that git
/// ignores, that the patterns name as folders, and that git ignores whole or are inside a folder it ignores whole; of
/// those, only the outermost. A file is never linked.
///
/// Feature: worktree-setup.folders-to-link
#[must_use]
pub fn folders_to_link(input: &LinkInput) -> Vec<String> {
    let ignored: BTreeSet<&str> = input.ignored_folders.iter().map(String::as_str).collect();
    let named: BTreeSet<&str> = input.named_folders.iter().map(String::as_str).collect();
    let candidates = folders_holding_matches(&input.ignored_files, &input.matched_files);
    let linked: Vec<String> = candidates
        .into_iter()
        .filter(|folder| ignored.contains(folder.as_str()) && named.contains(folder.as_str()))
        .filter(|folder| {
            input
                .ignored_whole
                .iter()
                .any(|whole| whole == folder || is_inside(folder, whole))
        })
        .collect();
    linked
        .iter()
        .filter(|folder| !linked.iter().any(|outer| is_inside(folder, outer)))
        .cloned()
        .collect()
}

/// The files and folders a new worktree gets as copies: the files git ignores that the patterns match, but for what
/// is inside a linked folder and what clashes with the files of the worktree's commit; a folder git ignores whole all
/// of whose files are copied is copied as one.
///
/// Feature: worktree-setup.paths-to-copy
#[must_use]
pub fn paths_to_copy(input: &CopyInput) -> Vec<String> {
    let matched: BTreeSet<&str> = input.matched_files.iter().map(String::as_str).collect();
    let copied: BTreeSet<&str> = input
        .ignored_files
        .iter()
        .map(String::as_str)
        .filter(|file| matched.contains(file))
        .filter(|file| !input.linked_folders.iter().any(|folder| is_inside(file, folder)))
        .filter(|file| !clashes(file, &input.commit_files))
        .collect();
    let mut paths: BTreeSet<String> = BTreeSet::new();
    let mut taken_whole: Vec<&str> = Vec::new();
    for whole in &input.ignored_whole {
        let mut files = input
            .ignored_files
            .iter()
            .filter(|file| is_inside(file, whole))
            .peekable();
        let is_all_copied = files.peek().is_some() && files.all(|file| copied.contains(file.as_str()));
        if is_all_copied && !clashes(whole, &input.commit_files) {
            let _ = paths.insert(whole.clone());
            taken_whole.push(whole);
        }
    }
    paths.extend(
        copied
            .into_iter()
            .filter(|file| !taken_whole.iter().any(|whole| is_inside(file, whole)))
            .map(str::to_owned),
    );
    paths.into_iter().collect()
}

/// Whether a path clashes with the files of a commit: one of them has its path, is inside it, or holds it.
fn clashes(path: &str, commit_files: &[String]) -> bool {
    commit_files
        .iter()
        .any(|file| file == path || is_inside(file, path) || is_inside(path, file))
}

/// Whether a path is inside a folder, not the folder itself.
fn is_inside(path: &str, folder: &str) -> bool {
    path.strip_prefix(folder).is_some_and(|rest| rest.starts_with('/'))
}

#[cfg(test)]
mod tests;
