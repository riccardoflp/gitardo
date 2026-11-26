//! Watching folders for changes: a thread for each folder waits for the system to say what changed under it, and hands
//! each batch of paths to the program. The program decides which changes count. The window of Gitardo Desktop watches
//! the repositories it shows with it, and `gitardo-engine` the git folders of those the extension has open (D93).

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

use crate::error::SystemError;

/// How often the thread of a folder looks whether it is to stop, in milliseconds.
#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
const STOP_CHECK: u32 = 100;

// The folder watched, as the system of the program watches it: with `ReadDirectoryChangesW` on Windows, with inotify
// on Linux, with `FSEvents` on macOS.
#[cfg(target_os = "linux")]
use crate::linux::DirectoryWatch;
#[cfg(target_os = "macos")]
use crate::macos::DirectoryWatch;
#[cfg(windows)]
use crate::windows::DirectoryWatch;

/// What a wait for the changes of a folder gave, on every system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Waited {
    /// These paths changed, from the folder watched.
    Changes(Vec<PathBuf>),
    /// More changed than the system could say: anything may have changed.
    Overflowed,
    /// Nothing changed in the time waited.
    Nothing,
    /// The folder cannot be watched any more, as when it was deleted.
    Gone,
}

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
    /// `SystemError::System` when a folder cannot be opened, and `SystemError::Unsupported` on a system whose part
    /// is not written yet.
    pub fn new(folders: &[PathBuf], deliver: &WatchDeliver) -> Result<Watcher, SystemError> {
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
                .map_err(|error| SystemError::system("spawning the folder watcher", &error))?;
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
#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
fn open(folder: &std::path::Path) -> Result<DirectoryWatch, SystemError> {
    DirectoryWatch::open(folder)
}

/// What the thread of a folder does until it is to stop or the folder is gone.
#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
fn run(mut watch: DirectoryWatch, folder: &std::path::Path, stop: &AtomicBool, deliver: &WatchDeliver) {
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

/// Watching folders is written for Windows, Linux and macOS only.
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn open(_folder: &std::path::Path) -> Result<(), SystemError> {
    Err(SystemError::Unsupported {
        what: "watch folders for changes yet",
    })
}

/// Watching folders is written for Windows, Linux and macOS only.
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn run(_watch: (), _folder: &std::path::Path, _stop: &AtomicBool, _deliver: &WatchDeliver) {}
