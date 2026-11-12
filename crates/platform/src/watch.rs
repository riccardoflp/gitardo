//! Watching folders for changes: a thread for each folder waits for the system to say what changed under it, and hands
//! each batch of paths to the program. The program decides which changes count.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

use crate::error::PlatformError;

/// How often the thread of a folder looks whether it is to stop, in milliseconds.
#[cfg(windows)]
const STOP_CHECK: u32 = 100;

/// What changed in a folder watched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WatchEvent {
    /// A file or a folder changed: created, deleted, renamed or written. The path is whole.
    Changed(PathBuf),
    /// More changed under a folder than the system could say: anything under it may have changed.
    Overflowed(PathBuf),
    /// A folder cannot be watched any more, as when it was deleted.
    Gone(PathBuf),
}

/// Where the changes go: called on the threads of the watcher, with each batch of changes.
pub type WatchDeliver = Arc<dyn Fn(Vec<WatchEvent>) + Send + Sync>;

/// The folders watched, each with everything under it, until the watcher is dropped.
pub struct Watcher {
    /// Set when the threads are to stop.
    stop: Arc<AtomicBool>,
    /// The threads, one for each folder.
    threads: Vec<JoinHandle<()>>,
}

impl Watcher {
    /// Watches folders and everything under them; each batch of changes goes to `deliver`.
    ///
    /// # Errors
    ///
    /// `PlatformError::System` when a folder cannot be opened, and `PlatformError::Unsupported` on a system whose part
    /// is not written yet.
    pub fn new(folders: &[PathBuf], deliver: &WatchDeliver) -> Result<Watcher, PlatformError> {
        let stop = Arc::new(AtomicBool::new(false));
        let mut threads = Vec::new();
        for folder in folders {
            let watch = open(folder)?;
            let stop = Arc::clone(&stop);
            let deliver = Arc::clone(deliver);
            let folder = folder.clone();
            let thread = thread::Builder::new()
                .name(String::from("folder watcher"))
                .spawn(move || run(watch, &folder, &stop, &deliver))
                .map_err(|error| PlatformError::system("spawning the folder watcher", &error))?;
            threads.push(thread);
        }
        Ok(Watcher { stop, threads })
    }
}

impl Drop for Watcher {
    /// Stops the threads, which give back what they asked of the system.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        for thread in self.threads.drain(..) {
            let _ended = thread.join();
        }
    }
}

/// Opens a folder to watch it.
#[cfg(windows)]
fn open(folder: &std::path::Path) -> Result<crate::windows::DirectoryWatch, PlatformError> {
    crate::windows::DirectoryWatch::open(folder)
}

/// What the thread of a folder does until it is to stop or the folder is gone.
#[cfg(windows)]
fn run(mut watch: crate::windows::DirectoryWatch, folder: &std::path::Path, stop: &AtomicBool, deliver: &WatchDeliver) {
    use crate::windows::Waited;
    while !stop.load(Ordering::SeqCst) {
        match watch.wait(STOP_CHECK) {
            Waited::Nothing => {}
            Waited::Changes(paths) => deliver(
                paths
                    .into_iter()
                    .map(|path| WatchEvent::Changed(folder.join(path)))
                    .collect(),
            ),
            Waited::Overflowed => deliver(vec![WatchEvent::Overflowed(folder.to_path_buf())]),
            Waited::Gone => {
                deliver(vec![WatchEvent::Gone(folder.to_path_buf())]);
                return;
            }
        }
    }
}

/// Watching folders is written for Windows only so far.
#[cfg(not(windows))]
fn open(_folder: &std::path::Path) -> Result<(), PlatformError> {
    Err(PlatformError::Unsupported {
        what: "watch folders for changes yet",
    })
}

/// Watching folders is written for Windows only so far.
#[cfg(not(windows))]
fn run(_watch: (), _folder: &std::path::Path, _stop: &AtomicBool, _deliver: &WatchDeliver) {}
