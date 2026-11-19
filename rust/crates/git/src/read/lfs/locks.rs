//! The locks of the files stored with Git LFS, asked of the server (`lfs.read-locks`): only when the **Large files**
//! page opens and on its **Refresh**, never with the reads of the repository (Q1195).

use std::ffi::OsString;

use model::lfs::{LfsLock, LfsLocks, LfsLocksRequest};

use super::json::{Json, parse};
use crate::read::fields::arguments;
use crate::{Git, GitError};

/// What Git LFS says of a server that does not support locks, in its English words.
const UNSUPPORTED: [&str; 3] = ["locking API", "does not support", "not supported"];

/// The arguments that list the locks: `git lfs locks --verify --json`, which says whose each lock is, or
/// `git lfs locks --json`, with `--remote=<remote>` when one is named.
#[must_use]
pub fn locks_arguments(request: &LfsLocksRequest, verify: bool) -> Vec<OsString> {
    let mut listed = arguments(&["lfs", "locks"]);
    if verify {
        listed.push(OsString::from("--verify"));
    }
    listed.push(OsString::from("--json"));
    if let Some(remote) = &request.remote {
        listed.push(OsString::from(format!("--remote={remote}")));
    }
    listed
}

/// The locks a server holds, by `git lfs locks --verify --json`; when the server does not verify, by
/// `git lfs locks --json`, with no word on whose they are. A server that does not support locks gives that fact and
/// what Git LFS said.
///
/// # Errors
///
/// When git cannot be started, is cancelled, or Git LFS fails for another reason.
///
/// Feature: lfs.read-locks
pub fn read_locks(git: &Git, request: &LfsLocksRequest) -> Result<LfsLocks, GitError> {
    let verified = git.read_output(&locks_arguments(request, true))?;
    if verified.is_success() {
        return Ok(parse_verified(&String::from_utf8_lossy(&verified.stdout)));
    }
    let plain_arguments = locks_arguments(request, false);
    let plain = git.read_output(&plain_arguments)?;
    if plain.is_success() {
        return Ok(parse_plain(&String::from_utf8_lossy(&plain.stdout)));
    }
    let said = String::from_utf8_lossy(&plain.stderr).trim().to_owned();
    if let Some(unsupported) = unsupported(&said) {
        return Ok(unsupported);
    }
    Err(GitError::Failed {
        command: crate::command_line(&plain_arguments),
        code: plain.code,
        stderr: said,
    })
}

/// That the server does not support locks, when what Git LFS said when it failed says so; none for another failure.
/// A server that answers `404` is taken by Git LFS 3.7.1 for one with no lock, and is not told apart.
#[must_use]
pub fn unsupported(said: &str) -> Option<LfsLocks> {
    UNSUPPORTED
        .iter()
        .any(|words| said.contains(words))
        .then(|| LfsLocks::Unsupported(said.to_owned()))
}

/// The locks `git lfs locks --verify --json` printed: `{"ours":[…],"theirs":[…]}`.
#[must_use]
pub fn parse_verified(printed: &str) -> LfsLocks {
    let Some(value) = parse(printed.trim()) else {
        return LfsLocks::Read(Vec::new());
    };
    let mut locks = Vec::new();
    for (side, is_ours) in [("ours", true), ("theirs", false)] {
        let listed = value.get(side).map(Json::items).unwrap_or_default();
        locks.extend(listed.iter().filter_map(|lock| lock_of(lock, Some(is_ours))));
    }
    LfsLocks::Read(locks)
}

/// The locks `git lfs locks --json` printed: an array of them, with no word on whose they are.
#[must_use]
pub fn parse_plain(printed: &str) -> LfsLocks {
    let locks = parse(printed.trim())
        .map(|value| value.items().iter().filter_map(|lock| lock_of(lock, None)).collect())
        .unwrap_or_default();
    LfsLocks::Read(locks)
}

/// A lock of the JSON of Git LFS: `{"id":"1","path":"a.psd","owner":{"name":"reviewer"},"locked_at":"…"}`.
fn lock_of(value: &Json, is_ours: Option<bool>) -> Option<LfsLock> {
    let text = |name: &str| value.get(name).and_then(Json::text).map(str::to_owned);
    Some(LfsLock {
        id: text("id")?,
        path: text("path")?,
        owner: value
            .get("owner")
            .and_then(|owner| owner.get("name"))
            .and_then(Json::text)
            .unwrap_or_default()
            .to_owned(),
        locked_at: text("locked_at").unwrap_or_default(),
        is_ours,
    })
}
