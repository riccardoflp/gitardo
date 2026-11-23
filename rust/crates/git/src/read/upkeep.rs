//! The upkeep of a repository: the room its objects take (`upkeep.read-size`), whether git's scheduled maintenance
//! looks after it (`upkeep.read-background`), and whether git keeps its commit-graph file of it
//! (`upkeep.read-commit-graph`).

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::SystemTime;

use model::upkeep::{CommitGraph, RepositorySize, is_registered};

use crate::read::fields::arguments;
use crate::{Git, GitError};

/// The repository whose size is read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizeRequest {
    /// Its folder.
    pub folder: PathBuf,
}

/// The repository asked about, and what the read before found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundRequest {
    /// Its folder.
    pub folder: PathBuf,
    /// What the read before found, with the stamp of the user's configuration it read; read again only when that
    /// changed.
    pub before: Option<BackgroundRead>,
}

/// Whether git's maintenance looks after the repository, with the stamp of the user's configuration it was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundRead {
    /// For each file of the user's configuration, its size and when it was last written; none when it is not there.
    stamp: Vec<Option<(u64, Option<SystemTime>)>>,
    /// Whether `maintenance.repo` lists the repository.
    pub is_registered: bool,
}

/// The repository asked about, and what the read before found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitGraphRequest {
    /// Its folder.
    pub folder: PathBuf,
    /// What the read before found, with the stamp of the object folder it read; read again only when that changed.
    pub before: Option<CommitGraphRead>,
}

/// Whether git keeps its commit-graph file of the repository, with where git keeps it and the stamp of those places
/// when it was read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitGraphRead {
    /// The two places of the file, alone and as a chain, in full.
    places: Vec<PathBuf>,
    /// For the files and the folders that say whether the file is there and whether git reads it, their size and when
    /// they were last written; none for one that is not there.
    stamp: Vec<Option<(u64, Option<SystemTime>)>>,
    /// What was read.
    pub graph: CommitGraph,
}

/// `git count-objects -v`, without `-H`, whose sizes are for people.
#[must_use]
pub fn size_arguments() -> Vec<OsString> {
    arguments(&["count-objects", "-v"])
}

/// `git config --global --get-all maintenance.repo`: the repositories registered, one a line.
#[must_use]
pub fn background_arguments() -> Vec<OsString> {
    arguments(&["config", "--global", "--get-all", "maintenance.repo"])
}

/// `git rev-parse --git-path objects/info/commit-graph --git-path objects/info/commit-graphs/commit-graph-chain`: the
/// two places git keeps its commit-graph file, alone or as a chain, one a line; in a worktree, those of the repository
/// the worktrees share.
#[must_use]
pub fn commit_graph_arguments() -> Vec<OsString> {
    arguments(&[
        "rev-parse",
        "--git-path",
        "objects/info/commit-graph",
        "--git-path",
        "objects/info/commit-graphs/commit-graph-chain",
    ])
}

/// `git config --type=bool --get core.commitGraph`: `false` when git does not read the file; nothing when it is not set.
#[must_use]
pub fn commit_graph_setting_arguments() -> Vec<OsString> {
    arguments(&["config", "--type=bool", "--get", "core.commitGraph"])
}

/// The places `git rev-parse --git-path` printed, one a line, each in full: a place git gives from the folder it ran in
/// is put after that folder.
#[must_use]
pub fn parse_commit_graph_places(output: &[u8], folder: &std::path::Path) -> Vec<PathBuf> {
    String::from_utf8_lossy(output)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| folder.join(line))
        .collect()
}

/// Whether what `git config --type=bool --get core.commitGraph` printed says git does not read the file.
#[must_use]
pub fn parse_commit_graph_setting(output: &[u8]) -> bool {
    String::from_utf8_lossy(output).trim() == "false"
}

/// The size `git count-objects -v` printed: lines `<key>: <value>`; a key it does not know is left aside.
#[must_use]
pub fn parse_size(output: &[u8]) -> RepositorySize {
    let text = String::from_utf8_lossy(output);
    let mut size = RepositorySize::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let Ok(value) = value.trim().parse::<u64>() else {
            continue;
        };
        match key.trim() {
            "count" => size.loose = value,
            "size" => size.loose_kib = value,
            "in-pack" => size.packed = value,
            "packs" => size.packs = value,
            "size-pack" => size.pack_kib = value,
            "prune-packable" => size.prune_packable = value,
            "size-garbage" => size.garbage_kib = value,
            _ => {}
        }
    }
    size
}

/// How much room the objects of the repository take. It runs when asked, never with the reads of the repository.
///
/// Feature: upkeep.read-size
///
/// # Errors
///
/// When git cannot be started or fails.
pub fn read_size(git: &Git, request: &SizeRequest) -> Result<RepositorySize, GitError> {
    Ok(parse_size(
        &git.in_directory(&request.folder).run_read(&size_arguments())?,
    ))
}

/// Whether git's scheduled maintenance looks after the repository: `maintenance.repo` in the user's configuration lists
/// its folder. None set reads as none registered.
///
/// Feature: upkeep.read-background
///
/// # Errors
///
/// When git cannot be started, or fails otherwise than by finding nothing set.
pub fn read_background(git: &Git, request: &BackgroundRequest) -> Result<bool, GitError> {
    read_background_kept(git, request).map(|read| read.is_registered)
}

/// [`read_background`], kept from the read before while no file of the user's configuration changed since.
///
/// # Errors
///
/// When git cannot be started, or fails otherwise than by finding nothing set.
pub fn read_background_kept(git: &Git, request: &BackgroundRequest) -> Result<BackgroundRead, GitError> {
    let stamp = configuration_stamp();
    if let Some(before) = request.before.as_ref().filter(|before| before.stamp == stamp) {
        return Ok(before.clone());
    }
    let output = git.in_directory(&request.folder).read_output(&background_arguments())?;
    // Git ends with 1 when nothing is set, and reads it as none registered.
    let listed: Vec<String> = if output.is_success() {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_owned)
            .collect()
    } else if output.code == Some(1) {
        Vec::new()
    } else {
        return Err(GitError::Failed {
            command: crate::command_line(&background_arguments()),
            code: output.code,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    };
    Ok(BackgroundRead {
        stamp,
        is_registered: is_registered(&listed, &request.folder),
    })
}

/// Whether git keeps its commit-graph file of the repository, alone or as a chain, and whether git reads it
/// (`core.commitGraph`). In a worktree it is the file of the repository the worktrees share.
///
/// Feature: upkeep.read-commit-graph
///
/// # Errors
///
/// When git cannot be started, or fails otherwise than by finding `core.commitGraph` not set.
pub fn read_commit_graph(git: &Git, request: &CommitGraphRequest) -> Result<CommitGraph, GitError> {
    read_commit_graph_kept(git, request).map(|read| read.graph)
}

/// [`read_commit_graph`], kept from the read before while the places of the file, the folders that hold them and the
/// configuration of the repository and of the user did not change since: git runs again only when they did.
///
/// # Errors
///
/// As [`read_commit_graph`].
pub fn read_commit_graph_kept(git: &Git, request: &CommitGraphRequest) -> Result<CommitGraphRead, GitError> {
    if let Some(before) = request
        .before
        .as_ref()
        .filter(|before| before.stamp == commit_graph_stamp(&before.places))
    {
        return Ok(before.clone());
    }
    let git = git.in_directory(&request.folder);
    let places = parse_commit_graph_places(&git.run_read(&commit_graph_arguments())?, &request.folder);
    let setting = git.read_output(&commit_graph_setting_arguments())?;
    // Git ends with 1 when the setting is not set, and then reads the file.
    let is_ignored = if setting.is_success() {
        parse_commit_graph_setting(&setting.stdout)
    } else if setting.code == Some(1) {
        false
    } else {
        return Err(GitError::Failed {
            command: crate::command_line(&commit_graph_setting_arguments()),
            code: setting.code,
            stderr: String::from_utf8_lossy(&setting.stderr).into_owned(),
        });
    };
    let stamp = commit_graph_stamp(&places);
    let has_file = places.iter().any(|place| place.is_file());
    Ok(CommitGraphRead {
        places,
        stamp,
        graph: CommitGraph { has_file, is_ignored },
    })
}

/// The stamp of what says whether the commit-graph file is there and whether git reads it: each place of the file and
/// the folder that holds it, the configuration of the repository, in the folder the object folder is in, and the user's.
fn commit_graph_stamp(places: &[PathBuf]) -> Vec<Option<(u64, Option<SystemTime>)>> {
    let mut files: Vec<PathBuf> = Vec::new();
    for place in places {
        files.push(place.clone());
        files.extend(place.parent().map(std::path::Path::to_path_buf));
    }
    let common = places
        .first()
        .and_then(|place| place.ancestors().nth(3))
        .map(|common| common.join("config"));
    files.extend(common);
    let mut stamp: Vec<_> = files.iter().map(|file| stamp_of(file)).collect();
    stamp.extend(configuration_stamp());
    stamp
}

/// The size of a file or a folder and when it was last written; none when it is not there.
fn stamp_of(file: &std::path::Path) -> Option<(u64, Option<SystemTime>)> {
    std::fs::metadata(file)
        .ok()
        .map(|metadata| (metadata.len(), metadata.modified().ok()))
}

/// The files of the user's configuration of git, each stamped: `GIT_CONFIG_GLOBAL` when set, else `~/.gitconfig` and
/// `$XDG_CONFIG_HOME/git/config` (or `~/.config/git/config`).
fn configuration_stamp() -> Vec<Option<(u64, Option<SystemTime>)>> {
    let files: Vec<PathBuf> = if let Some(global) = std::env::var_os("GIT_CONFIG_GLOBAL") {
        vec![PathBuf::from(global)]
    } else {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_default();
        let xdg = std::env::var_os("XDG_CONFIG_HOME").map_or_else(|| home.join(".config"), PathBuf::from);
        vec![home.join(".gitconfig"), xdg.join("git").join("config")]
    };
    files.iter().map(|file| stamp_of(file)).collect()
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{parse_commit_graph_places, parse_commit_graph_setting, parse_size};

    #[test]
    fn upkeep_read_size__the_keys_of_count_objects_give_the_loose_and_the_packed_objects() {
        let size = parse_size(b"count: 29\nsize: 1\nin-pack: 1200\npacks: 2\nsize-pack: 5120\nprune-packable: 0\n");
        assert_eq!((size.loose, size.loose_kib), (29, 1));
        assert_eq!((size.packed, size.packs, size.pack_kib), (1200, 2, 5120));
    }

    #[test]
    fn upkeep_read_size__a_key_it_does_not_know_is_left_aside() {
        let size = parse_size(b"count: 3\nsomething-new: 99\nsize-garbage: 4\n");
        assert_eq!((size.loose, size.garbage_kib, size.packs), (3, 4, 0));
    }

    #[test]
    fn upkeep_read_commit_graph__the_places_are_put_after_the_folder_unless_git_gives_them_in_full() {
        let folder = Path::new("/work/app");
        let places = parse_commit_graph_places(
            b".git/objects/info/commit-graph\n/work/app/.git/objects/info/commit-graphs/commit-graph-chain\n",
            folder,
        );
        assert_eq!(
            places,
            [
                PathBuf::from("/work/app/.git/objects/info/commit-graph"),
                PathBuf::from("/work/app/.git/objects/info/commit-graphs/commit-graph-chain"),
            ]
        );
        assert!(parse_commit_graph_setting(b"false\n"));
        assert!(!parse_commit_graph_setting(b"true\n"));
        assert!(!parse_commit_graph_setting(b""));
    }
}
