//! The upkeep of a repository: the room its objects take (`upkeep.read-size`), and whether git's scheduled maintenance
//! looks after it (`upkeep.read-background`).

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::SystemTime;

use model::upkeep::{RepositorySize, is_registered};

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
    files
        .iter()
        .map(|file| {
            std::fs::metadata(file)
                .ok()
                .map(|metadata| (metadata.len(), metadata.modified().ok()))
        })
        .collect()
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::parse_size;

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
}
