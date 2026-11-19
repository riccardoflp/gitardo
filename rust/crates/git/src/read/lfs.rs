//! What a repository stores with Git LFS (D84): whether Git LFS is installed and set up, the patterns of the files of
//! attributes, the files with their sizes, and the content of a pointer read from the store of the repository, never
//! downloaded by a read.

use std::path::{Path, PathBuf};

use model::lfs::{LfsContent, LfsContentRequest, LfsFile, LfsFilesRequest, LfsPattern, LfsStatus, LfsStatusRequest};

use crate::read::fields::arguments;
use crate::{Git, GitError};

#[cfg(test)]
mod tests;

/// Whether Git LFS is installed and its version, whether the repository is set up for it, and the patterns its files of
/// attributes store with it: those git tracks and the exclude file of attributes of the repository.
///
/// # Errors
///
/// When git cannot be started, is cancelled or fails.
///
/// Feature: lfs.read-status
pub fn read_status(git: &Git, request: &LfsStatusRequest) -> Result<LfsStatus, GitError> {
    let version = match &request.version {
        Some(version) => version.clone(),
        None => read_version(git)?,
    };
    let configured = git.read_output(&arguments(&["config", "--get-regexp", "^filter\\.lfs\\."]))?;
    let is_set_up = String::from_utf8_lossy(&configured.stdout).lines().any(|line| {
        let key = line.split_whitespace().next().unwrap_or_default();
        matches!(key, "filter.lfs.process" | "filter.lfs.smudge")
    });
    if let Some(patterns) = &request.patterns {
        return Ok(LfsStatus {
            version,
            is_set_up,
            patterns: patterns.clone(),
        });
    }
    let files = git.run_read(&arguments(&["ls-files", "-z", "--", ":(glob)**/.gitattributes"]))?;
    let mut patterns = Vec::new();
    for file in String::from_utf8_lossy(&files)
        .split('\0')
        .filter(|file| !file.is_empty())
    {
        let text = std::fs::read(git.directory().join(file)).unwrap_or_default();
        patterns.extend(lfs_patterns(&String::from_utf8_lossy(&text), file));
    }
    let info = git.run_read(&arguments(&["rev-parse", "--git-path", "info/attributes"]))?;
    let info = String::from_utf8_lossy(&info).trim().to_owned();
    if let Ok(text) = std::fs::read(git.directory().join(&info)) {
        patterns.extend(lfs_patterns(&String::from_utf8_lossy(&text), &info.replace('\\', "/")));
    }
    Ok(LfsStatus {
        version,
        is_set_up,
        patterns,
    })
}

/// `git lfs version`: the version of its first line, `git-lfs/3.7.1 (…)`; none when git says `lfs` is not one of its
/// commands.
fn read_version(git: &Git) -> Result<Option<String>, GitError> {
    let output = git.read_output(&arguments(&["lfs", "version"]))?;
    if !output.is_success() {
        return Ok(None);
    }
    Ok(parse_version(&String::from_utf8_lossy(&output.stdout)))
}

/// The version on the first line `git lfs version` prints: `3.7.1` of `git-lfs/3.7.1 (GitHub; …)`.
#[must_use]
pub fn parse_version(printed: &str) -> Option<String> {
    printed
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("git-lfs/"))
        .and_then(|rest| rest.split_whitespace().next())
        .map(str::to_owned)
}

/// The patterns of a file of attributes whose attributes have `filter=lfs`, with their lines.
#[must_use]
pub fn lfs_patterns(text: &str, file: &str) -> Vec<LfsPattern> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let mut words = line.split_whitespace();
            let pattern = words.next().filter(|pattern| !pattern.starts_with('#'))?;
            words.any(|attribute| attribute == "filter=lfs").then(|| LfsPattern {
                pattern: pattern.to_owned(),
                file: file.to_owned(),
                line: index + 1,
            })
        })
        .collect()
}

/// The files of the commit checked out stored with Git LFS: `git lfs ls-files --json`, or with a Git LFS older than 3.0
/// `git lfs ls-files --long --size`.
///
/// # Errors
///
/// When git cannot be started, is cancelled or fails.
///
/// Feature: lfs.read-files
pub fn read_files(git: &Git, request: &LfsFilesRequest) -> Result<Vec<LfsFile>, GitError> {
    if request.version.is_empty() {
        return Ok(Vec::new());
    }
    let major: u32 = request
        .version
        .split('.')
        .next()
        .and_then(|major| major.parse().ok())
        .unwrap_or(0);
    if major >= 3 {
        let printed = git.run_read(&arguments(&["lfs", "ls-files", "--json"]))?;
        Ok(parse_files_json(&String::from_utf8_lossy(&printed)))
    } else {
        let printed = git.run_read(&arguments(&["lfs", "ls-files", "--long", "--size"]))?;
        Ok(parse_files_long(&String::from_utf8_lossy(&printed)))
    }
}

/// The files `git lfs ls-files --json` printed: an object for each file, one key on each line, as Git LFS indents it.
#[must_use]
pub fn parse_files_json(printed: &str) -> Vec<LfsFile> {
    let mut files = Vec::new();
    let mut current: Option<LfsFile> = None;
    for line in printed.lines().map(str::trim) {
        let Some((key, value)) = line.split_once("\": ") else {
            if line.starts_with('}')
                && let Some(file) = current.take()
            {
                files.push(file);
            }
            continue;
        };
        let file = current.get_or_insert_with(|| LfsFile {
            path: String::new(),
            size: 0,
            oid: String::new(),
            is_downloaded: false,
            is_checked_out: false,
        });
        let value = value.trim_end_matches(',');
        match key.trim_start_matches('"') {
            "name" => file.path = json_string(value),
            "size" => file.size = value.parse().unwrap_or(0),
            "oid" => file.oid = json_string(value),
            "downloaded" => file.is_downloaded = value == "true",
            "checkout" => file.is_checked_out = value == "true",
            _ => {}
        }
    }
    files
}

/// A string of JSON as text: its quotes taken away and its escapes read.
fn json_string(value: &str) -> String {
    let inner = value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(value);
    let mut text = String::new();
    let mut characters = inner.chars();
    let mut pending_high: Option<u32> = None;
    while let Some(character) = characters.next() {
        if character != '\\' {
            text.push(character);
            continue;
        }
        match characters.next() {
            Some('n') => text.push('\n'),
            Some('t') => text.push('\t'),
            Some('u') => {
                let digits: String = characters.by_ref().take(4).collect();
                let code = u32::from_str_radix(&digits, 16).unwrap_or(0xfffd);
                match (pending_high.take(), code) {
                    (None, 0xd800..=0xdbff) => pending_high = Some(code),
                    (Some(high), 0xdc00..=0xdfff) => {
                        let joined = 0x10000 + ((high - 0xd800) << 10) + (code - 0xdc00);
                        text.push(char::from_u32(joined).unwrap_or('\u{fffd}'));
                    }
                    (_, code) => text.push(char::from_u32(code).unwrap_or('\u{fffd}')),
                }
            }
            Some(other) => text.push(other),
            None => {}
        }
    }
    text
}

/// The files `git lfs ls-files --long --size` printed: `<oid> <*|-> <path> (<size>)`, `*` for a file downloaded; the
/// size as Git LFS rounds it, `1.2 KB`, in bytes.
#[must_use]
pub fn parse_files_long(printed: &str) -> Vec<LfsFile> {
    printed
        .lines()
        .filter_map(|line| {
            let (oid, rest) = line.split_once(' ')?;
            let (mark, rest) = rest.split_once(' ')?;
            let open = rest.rfind(" (")?;
            let (path, size) = (rest.get(..open)?, rest.get(open + 2..)?.trim_end_matches(')'));
            let is_downloaded = mark == "*";
            Some(LfsFile {
                path: path.to_owned(),
                size: rounded_size(size),
                oid: oid.to_owned(),
                is_downloaded,
                is_checked_out: is_downloaded,
            })
        })
        .collect()
}

/// A size as Git LFS rounds it, `1.2 KB`, in bytes, its units of 1,000.
fn rounded_size(size: &str) -> u64 {
    let (number, unit) = size.split_once(' ').unwrap_or((size, "B"));
    let times: f64 = match unit {
        "KB" => 1e3,
        "MB" => 1e6,
        "GB" => 1e9,
        "TB" => 1e12,
        _ => 1.0,
    };
    let bytes = number.parse::<f64>().unwrap_or(0.0) * times;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a size is positive and rounded to whole bytes"
    )]
    let whole = bytes.round().max(0.0) as u64;
    whole
}

/// The content a pointer stands for, read from the store of Git LFS of the repository (`git lfs env`, its line
/// `LocalMediaDir=`): the file of its id, whose size is checked against the pointer. It never runs the filter of Git
/// LFS, which would download from the server.
///
/// # Errors
///
/// When git cannot be started, is cancelled or fails.
///
/// Feature: lfs.read-content
pub fn read_content(git: &Git, request: &LfsContentRequest) -> Result<LfsContent, GitError> {
    let printed = git.run_read(&arguments(&["lfs", "env"]))?;
    let printed = String::from_utf8_lossy(&printed);
    let Some(folder) = printed.lines().find_map(|line| line.strip_prefix("LocalMediaDir=")) else {
        return Ok(LfsContent::NotDownloaded);
    };
    Ok(content_in(Path::new(folder.trim()), request))
}

/// The content of a pointer in a store of Git LFS: `<folder>/<first two digits>/<the next two>/<id>`, of the size the
/// pointer says.
#[must_use]
pub fn content_in(folder: &Path, request: &LfsContentRequest) -> LfsContent {
    let oid = &request.pointer.oid;
    let (Some(first), Some(second)) = (oid.get(..2), oid.get(2..4)) else {
        return LfsContent::NotDownloaded;
    };
    let path: PathBuf = folder.join(first).join(second).join(oid);
    match std::fs::read(path) {
        Ok(bytes) if u64::try_from(bytes.len()).ok() == Some(request.pointer.size) => LfsContent::Downloaded(bytes),
        _ => LfsContent::NotDownloaded,
    }
}

/// What the reads of the repository know of Git LFS (`repo.read-state`): whether it is set up and the patterns, and
/// the version of Git LFS only once the repository has patterns or is set up, so that a repository that stores
/// nothing with it starts no process of Git LFS.
///
/// # Errors
///
/// When git cannot be started, is cancelled or fails.
pub fn read_repository_status(git: &Git) -> Result<LfsStatus, GitError> {
    let without_version = LfsStatusRequest {
        version: Some(None),
        patterns: None,
    };
    let status = read_status(git, &without_version)?;
    if status.patterns.is_empty() && !status.is_set_up {
        return Ok(status);
    }
    Ok(LfsStatus {
        version: read_version(git)?,
        ..status
    })
}

/// The arguments that ask git whether the repository stores a path with Git LFS: `git check-attr -z filter -- <path>`.
#[must_use]
pub fn is_stored_arguments(path: &str) -> Vec<std::ffi::OsString> {
    arguments(&["check-attr", "-z", "filter", "--", path])
}

/// Whether what `git check-attr -z filter` printed says `lfs`: `<path>\0filter\0lfs\0`.
#[must_use]
pub fn parse_is_stored(printed: &[u8]) -> bool {
    let fields: Vec<&[u8]> = printed.split(|byte| *byte == 0).collect();
    fields.chunks(3).any(|field| field.get(2) == Some(&b"lfs".as_slice()))
}

/// Whether the repository stores a path with Git LFS, as its attributes say (`lfs.show-in-pages`): a file whose bytes
/// only look like a pointer at another path is shown as it is.
///
/// # Errors
///
/// When git cannot be started, is cancelled or fails.
pub fn read_is_stored(git: &Git, path: &str) -> Result<bool, GitError> {
    Ok(parse_is_stored(&git.run_read(&is_stored_arguments(path))?))
}
