//! A folder of its own for each test, under the temporary folder, deleted when the test is done with it.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How many times deleting the folder is tried again, when a file in it is still held for a moment.
const REMOVAL_ATTEMPTS: u32 = 5;

/// A folder under the temporary folder that no other test uses, deleted with what it holds when it is dropped.
#[derive(Debug)]
pub(crate) struct TemporaryFolder {
    path: PathBuf,
}

impl TemporaryFolder {
    /// Makes a new folder named `gitardo-test-<process>-<number>`, with the first number that no folder has yet.
    /// Creating a folder fails when it exists, so two tests running at the same time never get the same one, without a
    /// counter shared between them or a clock.
    pub(crate) fn new() -> TemporaryFolder {
        let parent = std::env::temp_dir();
        let process = std::process::id();
        for number in 0_u32.. {
            let path = parent.join(format!("gitardo-test-{process}-{number}"));
            match fs::create_dir(&path) {
                Ok(()) => return TemporaryFolder { path },
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
                // On Windows a folder of the same name that another test is still deleting refuses access: the next
                // name is as good.
                Err(error) if cfg!(windows) && error.kind() == ErrorKind::PermissionDenied && number < 10_000 => {}
                Err(error) => panic!("could not make a test folder at {}: {error}", path.display()),
            }
        }
        panic!("every test folder name under {} is taken", parent.display());
    }

    /// Where the folder is.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TemporaryFolder {
    fn drop(&mut self) {
        remove(&self.path);
    }
}

/// Deletes a folder and what it holds, as well as it can, without ever failing: a folder left behind in the temporary
/// folder does no harm to the test, and a panic while dropping would hide the reason a test failed.
fn remove(path: &Path) {
    if fs::remove_dir_all(path).is_ok() {
        return;
    }
    // Git makes the files under .git/objects read-only, and Windows refuses to delete a read-only file.
    clear_read_only(path);
    for attempt in 1..=REMOVAL_ATTEMPTS {
        match fs::remove_dir_all(path) {
            Ok(()) => return,
            Err(error) if error.kind() == ErrorKind::NotFound => return,
            // On Windows a git process that just ended, or an antivirus, may hold a file for a moment.
            Err(_) => std::thread::sleep(Duration::from_millis(u64::from(attempt) * 50)),
        }
    }
}

/// Clears the read-only flag of every file and folder under a folder. What cannot be cleared is left as it is: the
/// deletion that follows says whether it mattered.
fn clear_read_only(path: &Path) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        // A link is left alone, and not followed: what it points to is not the test's.
        let (Ok(kind), Ok(metadata)) = (entry.file_type(), entry.metadata()) else {
            continue;
        };
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            clear_read_only(&entry.path());
        }
        let mut permissions = metadata.permissions();
        if permissions.readonly() {
            // On Windows this clears the read-only attribute; on other systems it makes the file writable for
            // everyone, which does not matter for a file deleted right after.
            #[allow(clippy::permissions_set_readonly_false)]
            permissions.set_readonly(false);
            // A file whose flag stays set is reported by the deletion that follows.
            let _ = fs::set_permissions(entry.path(), permissions);
        }
    }
}
