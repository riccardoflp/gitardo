//! The locks of the files stored with Git LFS (`lfs.read-locks`, `lfs.lock`, `lfs.unlock`): who holds a file on the
//! server so that no one else changes it.

/// What reading the locks needs: the remote whose server is asked, or the one Git LFS chooses.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LfsLocksRequest {
    /// The remote, by its name; none for the one Git LFS chooses.
    pub remote: Option<String>,
}

/// A lock of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LfsLock {
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

/// The locks a server gave, or that it does not support them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LfsLocks {
    /// The locks.
    Read(Vec<LfsLock>),
    /// The server does not support locks: what Git LFS said.
    Unsupported(String),
}

impl LfsLocks {
    /// The lock of a file, if it has one.
    #[must_use]
    pub fn of(&self, path: &str) -> Option<&LfsLock> {
        match self {
            LfsLocks::Read(locks) => locks.iter().find(|lock| lock.path == path),
            LfsLocks::Unsupported(_) => None,
        }
    }
}
