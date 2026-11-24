//! The reads of Git LFS, which version 1.13 adds for milestone M47: whether Git LFS is there and what the repository
//! stores with it (`lfs.read-status`), which also comes with the state; the files stored with it (`lfs.read-files`);
//! the locks of the server (`lfs.read-locks`); and a file as a commit or the index has it with the content in place of
//! its pointer when the store of the repository has it (`lfs.read-content`, `lfs.show-in-pages`), never downloaded by
//! the read.

use model::lfs::{LfsFile, LfsLock, LfsLocks, LfsPattern, LfsPointer, LfsStatus};
use serde::{Deserialize, Serialize};

use super::BytesJson;

/// Whether Git LFS is installed and set up for a repository, and the patterns of the files it stores with it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LfsStatusJson {
    /// The version of Git LFS, `3.7.1`; none when it is not installed, or, in the state, when the repository has no
    /// pattern and is not set up, which starts no process of Git LFS.
    pub version: Option<String>,
    /// Whether the repository is set up: `filter.lfs.process` or `filter.lfs.smudge` is set.
    pub is_set_up: bool,
    /// The patterns, in the order of their files and lines.
    pub patterns: Vec<LfsPatternJson>,
}

/// A pattern of a file of attributes whose attributes have `filter=lfs`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LfsPatternJson {
    /// The pattern: `*.psd`.
    pub pattern: String,
    /// The file of attributes it is in, from the top of the working tree: `.gitattributes`.
    pub file: String,
    /// Its line in that file, from 1.
    pub line: usize,
}

impl From<&LfsStatus> for LfsStatusJson {
    fn from(status: &LfsStatus) -> LfsStatusJson {
        LfsStatusJson {
            version: status.version.clone(),
            is_set_up: status.is_set_up,
            patterns: status.patterns.iter().map(LfsPatternJson::from).collect(),
        }
    }
}

impl From<&LfsPattern> for LfsPatternJson {
    fn from(pattern: &LfsPattern) -> LfsPatternJson {
        LfsPatternJson {
            pattern: pattern.pattern.clone(),
            file: pattern.file.clone(),
            line: pattern.line,
        }
    }
}

/// The parameters of `read/lfs.read-files`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LfsFilesParams {
    /// The version of Git LFS `read/lfs.read-status` read, which tells which output of `git lfs ls-files` to ask for.
    pub version: String,
}

/// The answer of `read/lfs.read-files`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LfsFilesJson {
    /// The files of the commit checked out that the repository stores with Git LFS.
    pub files: Vec<LfsFileJson>,
}

/// A file stored with Git LFS.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LfsFileJson {
    /// Its path, from the top of the working tree, with `/`.
    pub path: String,
    /// The size of its content, in bytes.
    pub size: u64,
    /// The id of its content.
    pub oid: String,
    /// Whether its content is in the store of the repository.
    pub is_downloaded: bool,
    /// Whether the file on disk is its content rather than its pointer.
    pub is_checked_out: bool,
}

impl From<&LfsFile> for LfsFileJson {
    fn from(file: &LfsFile) -> LfsFileJson {
        LfsFileJson {
            path: file.path.clone(),
            size: file.size,
            oid: file.oid.clone(),
            is_downloaded: file.is_downloaded,
            is_checked_out: file.is_checked_out,
        }
    }
}

/// The parameters of `read/lfs.read-locks`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LfsLocksParams {
    /// The remote whose server is asked; none for the one Git LFS chooses.
    #[serde(default)]
    pub remote: Option<String>,
}

/// The answer of `read/lfs.read-locks`: the locks, or that the server does not support them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum LfsLocksJson {
    /// The locks the server gave.
    Read {
        /// Each lock.
        locks: Vec<LfsLockJson>,
    },
    /// The server does not support locks.
    Unsupported {
        /// What Git LFS said.
        message: String,
    },
}

/// A lock of a file, which `lfs.unlock` takes as its target.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LfsLockJson {
    /// Its id on the server.
    pub id: String,
    /// The file, from the top of the working tree, with `/`.
    pub path: String,
    /// The name of whoever holds it, as the server says it.
    pub owner: String,
    /// When it was locked, as the server says it.
    pub locked_at: String,
    /// Whether this user holds it; none when the server did not say whose it is.
    pub is_ours: Option<bool>,
}

impl From<&LfsLocks> for LfsLocksJson {
    fn from(locks: &LfsLocks) -> LfsLocksJson {
        match locks {
            LfsLocks::Read(locks) => LfsLocksJson::Read {
                locks: locks.iter().map(LfsLockJson::from).collect(),
            },
            LfsLocks::Unsupported(message) => LfsLocksJson::Unsupported {
                message: message.clone(),
            },
        }
    }
}

impl From<&LfsLock> for LfsLockJson {
    fn from(lock: &LfsLock) -> LfsLockJson {
        LfsLockJson {
            id: lock.id.clone(),
            path: lock.path.clone(),
            owner: lock.owner.clone(),
            locked_at: lock.locked_at.clone(),
            is_ours: lock.is_ours,
        }
    }
}

impl From<LfsLockJson> for LfsLock {
    fn from(lock: LfsLockJson) -> LfsLock {
        LfsLock {
            id: lock.id,
            path: lock.path,
            owner: lock.owner,
            locked_at: lock.locked_at,
            is_ours: lock.is_ours,
        }
    }
}

/// The parameters of `read/lfs.read-content`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LfsContentParams {
    /// The file, from the top of the working tree, with `/`.
    pub path: String,
    /// The commit, by any name git understands; none for the version the index has.
    #[serde(default)]
    pub revision: Option<String>,
}

/// The answer of `read/lfs.read-content`: a file as a commit or the index has it, with the content in place of its
/// pointer when it is one at a path the repository stores with Git LFS and the store of the repository has it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LfsContentJson {
    /// The bytes: the content, or the pointer when its content is not downloaded; none when the commit or the index
    /// has no such file.
    pub bytes: Option<BytesJson>,
    /// The commit the revision named, by its full id; none for the index, or a name that is no commit.
    pub commit: Option<String>,
    /// The pointer the file is, when it is one at a path the repository stores with Git LFS.
    pub stored: Option<StoredPointerJson>,
}

/// A version of a file that is a pointer of Git LFS.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredPointerJson {
    /// The size of its content, in bytes.
    pub size: u64,
    /// The id of its content, 64 hexadecimal digits.
    pub oid: String,
    /// Whether its content is in the store of the repository, and so in `bytes`.
    pub is_downloaded: bool,
}

impl StoredPointerJson {
    /// The JSON of a pointer, its content downloaded or not.
    #[must_use]
    pub fn of(pointer: &LfsPointer, is_downloaded: bool) -> StoredPointerJson {
        StoredPointerJson {
            size: pointer.size,
            oid: pointer.oid.clone(),
            is_downloaded,
        }
    }
}
