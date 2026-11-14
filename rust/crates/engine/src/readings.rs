//! Answering the queries of patches, files and comparisons: a patch of commits, what applying one would do, the bytes
//! of a file, a temporary file to write, and the changes of a comparison as a patch.

use std::hash::{BuildHasher, Hasher};
use std::path::PathBuf;

use git::read::compare::{ComparePatchRequest, CompareRequest, PatchForm, compare_patch_arguments, read_patch};
use git::read::patch::{FormatPatchRequest, PatchCheckRequest, PatchSource, check, read_format_patch};
use git::{Git, GitError};
use model::CommitId;
use model::compare::{ComparePatch, ComparisonTarget};
use model::contract::{Answer, Query};
use model::patch::PatchInput;

/// One patch of commits, oldest first.
pub(crate) fn format_patch(git: &Git, commits: &[CommitId]) -> Result<Answer, GitError> {
    let request = FormatPatchRequest {
        commits: commits.to_vec(),
    };
    read_format_patch(git, &request).map(Answer::Patch)
}

/// What `git apply --check --3way` says of a patch.
pub(crate) fn patch_check(git: &Git, patch: &PatchInput) -> Result<Answer, GitError> {
    let request = PatchCheckRequest { patch: source(patch) };
    check(git, &request).map(Answer::PatchCheck)
}

/// The bytes of a file.
pub(crate) fn file_contents(path: &std::path::Path) -> Result<Answer, GitError> {
    std::fs::read(path)
        .map(Answer::Contents)
        .map_err(|source| GitError::File {
            path: path.to_path_buf(),
            source,
        })
}

/// The bytes of files, each none when it is not there.
pub(crate) fn file_texts(paths: &[std::path::PathBuf]) -> Result<Answer, GitError> {
    let mut texts = Vec::with_capacity(paths.len());
    for path in paths {
        match std::fs::read(path) {
            Ok(bytes) => texts.push(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => texts.push(None),
            Err(source) => {
                return Err(GitError::File {
                    path: path.clone(),
                    source,
                });
            }
        }
    }
    Ok(Answer::Files(texts))
}

/// A file in the temporary folder of the system that no other has: `gitardo-<process>-<random>.<extension>`, the random
/// part from the keys the standard library draws for each hash map, which differ from one call to the next.
/// Whether something is at a path: a file, or a folder with something in it. A folder that cannot be read counts as
/// one with something in it, so that nothing is cloned over it.
pub(crate) fn has_files(path: &std::path::Path) -> bool {
    match std::fs::read_dir(path) {
        Ok(mut entries) => entries.next().is_some(),
        Err(_) => path.exists(),
    }
}

pub(crate) fn temporary_file(extension: &str) -> Answer {
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u32(std::process::id());
    let random = hasher.finish();
    let name = format!("gitardo-{}-{random:016x}.{extension}", std::process::id());
    Answer::Path(std::env::temp_dir().join(name))
}

/// The changes of a comparison as a patch that `git apply` takes, binary files included.
pub(crate) fn compare_patch(git: &Git, target: &ComparisonTarget) -> Result<Answer, GitError> {
    match read_patch(git, &patch_request(target))? {
        ComparePatch::Text(bytes) => Ok(Answer::Contents(bytes)),
        ComparePatch::Lines(_) => Ok(Answer::Contents(Vec::new())),
    }
}

/// The command a comparison's patch is read with, as the log writes it at the debug level.
pub(crate) fn describe_compare_patch(target: &ComparisonTarget) -> String {
    git::command_line(&compare_patch_arguments(&patch_request(target)))
}

/// The patch to copy of a comparison, as the page compares it.
fn patch_request(target: &ComparisonTarget) -> ComparePatchRequest {
    ComparePatchRequest {
        comparison: CompareRequest {
            first: target.first.side.clone(),
            second: target.second.side.clone(),
            mode: target.mode,
            path: target.path.clone(),
        },
        merge_base: target.merge_base.clone(),
        files: Vec::new(),
        form: PatchForm::Text,
    }
}

/// Where git finds a patch.
fn source(input: &PatchInput) -> PatchSource {
    match input {
        PatchInput::File(file) => PatchSource::File(PathBuf::from(file)),
        PatchInput::Text(text) => PatchSource::Text(text.clone()),
    }
}

/// What is read on disk: the texts of files; whether a path is there, a file, a folder or a link, a path the system
/// cannot look at being taken as one that is not there, since git says so when it runs; and the links of a worktree to
/// folders outside it.
pub(crate) fn on_disk(git: &Git, query: &Query) -> Result<Answer, GitError> {
    match query {
        Query::FileTexts { paths } => file_texts(paths),
        Query::FolderLinks { worktrees } => {
            let mut links = Vec::new();
            for worktree in worktrees {
                let request = git::read::worktree::FolderLinksRequest {
                    worktree: worktree.clone(),
                };
                links.extend(git::read::worktree::read_folder_links(git, &request)?);
            }
            Ok(Answer::Paths(links))
        }
        Query::PathExists { path } => Ok(Answer::Flag(path.symlink_metadata().is_ok())),
        _ => Ok(Answer::Flag(false)), // The engine sends only these here.
    }
}

/// What [`on_disk`] reads for the operations of worktrees, as the log writes it.
pub(crate) fn describe_worktree_reading(query: &Query) -> String {
    match query {
        Query::FolderLinks { worktrees } => worktrees
            .iter()
            .map(|worktree| {
                format!(
                    "git -C {} ls-files --others --directory -z, and with --ignored --exclude-standard",
                    worktree.display()
                )
            })
            .collect::<Vec<_>>()
            .join("; "),
        Query::PathExists { path } => format!("look for {}", path.display()),
        _ => String::new(), // The engine sends only these two here.
    }
}
