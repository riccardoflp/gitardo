//! Git's rerere (D84): whether it records and writes again the resolutions of conflicts in a repository
//! (`rerere.read-state`).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use model::rerere::{RerereState, RerereStateRequest};

use crate::read::fields::arguments;
use crate::{Git, GitError};

/// The arguments that read the two keys at once, as booleans:
/// `git config --type=bool --get-regexp ^rerere\.(enabled|autoupdate)$`.
#[must_use]
pub fn state_arguments() -> Vec<OsString> {
    arguments(&[
        "config",
        "--type=bool",
        "--get-regexp",
        r"^rerere\.(enabled|autoupdate)$",
    ])
}

/// The keys `git config --get-regexp` printed, one `<key> <value>` to a line, the key in lower case; the last of each
/// wins, as it does for git. `rerere.enabled` is none while it is not set.
#[must_use]
pub fn parse_state(printed: &str) -> (Option<bool>, Option<bool>) {
    let (mut enabled, mut stages) = (None, None);
    for line in printed.lines() {
        let Some((key, value)) = line.trim().split_once(' ') else {
            continue;
        };
        let value = value.trim() == "true";
        match key.to_ascii_lowercase().as_str() {
            "rerere.enabled" => enabled = Some(value),
            "rerere.autoupdate" => stages = Some(value),
            _ => {}
        }
    }
    (enabled, stages)
}

/// Whether git remembers resolutions: `rerere.enabled` when it is set; otherwise whether the folder `rr-cache` of the
/// repository exists, which git takes as on. With whether it stages what it writes, `rerere.autoUpdate`.
///
/// # Errors
///
/// When git cannot be started, or fails for another reason than finding neither key.
///
/// Feature: rerere.read-state
pub fn read_state(git: &Git, request: &RerereStateRequest) -> Result<RerereState, GitError> {
    let listed = state_arguments();
    let output = git.read_output(&listed)?;
    // `git config --get-regexp` ends with 1 when no key matches.
    if !output.is_success() && output.code != Some(1) {
        return Err(GitError::Failed {
            command: crate::command_line(&listed),
            code: output.code,
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    let (enabled, stages) = parse_state(&String::from_utf8_lossy(&output.stdout));
    let is_on = enabled.unwrap_or_else(|| rr_cache(&request.git_folder).is_dir());
    Ok(RerereState {
        is_on,
        stages: stages.unwrap_or(false),
        sets_stages: stages.is_some(),
    })
}

/// The folder `rr-cache` of a repository, as `git rev-parse --git-path rr-cache` gives it: in the common git folder,
/// which the file `commondir` of a linked worktree names, without starting git.
#[must_use]
pub fn rr_cache(git_folder: &Path) -> PathBuf {
    let common = std::fs::read_to_string(git_folder.join("commondir"))
        .ok()
        .map_or_else(|| git_folder.to_path_buf(), |named| git_folder.join(named.trim()));
    common.join("rr-cache")
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
