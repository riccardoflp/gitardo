//! Watching a folder and everything under it with a stream of `FSEvents`, which says each file that changed under it.
//! The stream calls back on a queue of its own; what it says is sent to the thread that waits, which waits with a
//! timeout, so that it can also look, a few times a second, whether it is to stop.

use core::ffi::{CStr, c_void};
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use super::core_services as fs;
use super::libsystem;
use super::objects;
use crate::error::SystemError;
use crate::watch::Waited;

/// How long the stream gathers changes before it calls back, in seconds: a moment, so that a checkout writing many
/// files is said in a few batches.
const LATENCY: f64 = 0.05;

/// What a change says was lost: everything under its path may have changed.
const LOST: u32 = fs::kFSEventStreamEventFlagMustScanSubDirs
    | fs::kFSEventStreamEventFlagUserDropped
    | fs::kFSEventStreamEventFlagKernelDropped;

/// What a change of a folder says that its name came or went, rather than only its times.
const NAMED: u32 = fs::kFSEventStreamEventFlagItemCreated
    | fs::kFSEventStreamEventFlagItemRemoved
    | fs::kFSEventStreamEventFlagItemRenamed;

/// One change the stream said: the whole path, and what it was.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Change {
    path: PathBuf,
    flags: u32,
}

/// Where the callback of the stream sends what it is told.
type Deliveries = Sender<Vec<Change>>;

/// A queue of `libdispatch`, released when dropped.
struct Queue(libsystem::dispatch_queue_t);

impl Drop for Queue {
    fn drop(&mut self) {
        // SAFETY: the queue was made for this value, which releases it once, after the stream that used it.
        unsafe { libsystem::dispatch_release(self.0) };
    }
}

/// A stream on its queue, stopped and released when dropped, before its queue.
struct Stream {
    events: fs::FSEventStreamRef,
    started: bool,
    _queue: Queue,
}

// SAFETY: the stream and its queue are owned by this value alone, which only the thread of the folder holds, and the
// calls of `FSEvents` and `libdispatch` on them may be made from any thread; the callback runs on the queue and shares
// only the deliveries, which are `Send` and `Sync`.
unsafe impl Send for Stream {}

impl Drop for Stream {
    fn drop(&mut self) {
        // SAFETY: the stream was made for this value and set on its queue; it is stopped if it was started, then
        // invalidated and released once, as `FSEvents` asks of a stream on a queue.
        unsafe {
            if self.started {
                fs::FSEventStreamStop(self.events);
            }
            fs::FSEventStreamInvalidate(self.events);
            fs::FSEventStreamRelease(self.events);
        }
    }
}

/// A folder watched.
pub(crate) struct DirectoryWatch {
    /// What the stream says comes here.
    received: Receiver<Vec<Change>>,
    /// The folder watched, as the system writes it, with every link resolved: what the paths of the stream start with.
    root: PathBuf,
    /// The stream, alive as long as the watch is.
    _stream: Stream,
}

impl DirectoryWatch {
    /// Opens a folder to watch it and everything under it. What changes from now on is said.
    pub(crate) fn open(folder: &Path) -> Result<DirectoryWatch, SystemError> {
        let root =
            std::fs::canonicalize(folder).map_err(|error| SystemError::system("opening a folder to watch", &error))?;
        let text = root.to_str().ok_or(SystemError::Unsupported {
            what: "watch a folder whose path is not UTF-8",
        })?;
        let path = objects::string(text)?;
        let paths = objects::array(&[&path])?;
        let (sender, received) = mpsc::channel();
        let deliveries = Arc::new(sender);
        // SAFETY: the label is a C string alive for the call; a null attribute asks for a queue that runs one piece of
        // work at a time.
        let queue = unsafe { libsystem::dispatch_queue_create(c"folder watcher".as_ptr(), core::ptr::null()) };
        if queue.is_null() {
            return Err(SystemError::system("dispatch_queue_create", &"no queue was made"));
        }
        let queue = Queue(queue);
        let context = fs::FSEventStreamContext {
            version: 0,
            info: Arc::as_ptr(&deliveries).cast_mut().cast::<c_void>(),
            retain: Some(retain),
            release: Some(release),
            copy_description: None,
        };
        let flags = fs::kFSEventStreamCreateFlagNoDefer
            | fs::kFSEventStreamCreateFlagWatchRoot
            | fs::kFSEventStreamCreateFlagFileEvents;
        // SAFETY: the context and the array of paths are alive for the call; the stream retains the array, and keeps
        // the deliveries through `retain` and `release`, which count it as one more owner of the `Arc`.
        let stream = unsafe {
            fs::FSEventStreamCreate(
                core::ptr::null(),
                told,
                &raw const context,
                paths.get(),
                fs::kFSEventStreamEventIdSinceNow,
                LATENCY,
                flags,
            )
        };
        if stream.is_null() {
            return Err(SystemError::system("FSEventStreamCreate", &"no stream was made"));
        }
        // SAFETY: the stream and the queue were just made, and the queue outlives the stream in `Stream`.
        unsafe { fs::FSEventStreamSetDispatchQueue(stream, queue.0) };
        let mut stream = Stream {
            events: stream,
            started: false,
            _queue: queue,
        };
        // SAFETY: the stream is set on its queue, as starting it asks.
        stream.started = unsafe { fs::FSEventStreamStart(stream.events) } != 0;
        if !stream.started {
            return Err(SystemError::system("FSEventStreamStart", &"the stream did not start"));
        }
        Ok(DirectoryWatch {
            received,
            root,
            _stream: stream,
        })
    }

    /// Waits at most some milliseconds for the changes of the folder.
    pub(crate) fn wait(&mut self, milliseconds: u32) -> Waited {
        let mut changes = match self.received.recv_timeout(Duration::from_millis(milliseconds.into())) {
            Ok(changes) => changes,
            Err(RecvTimeoutError::Timeout) => return Waited::Nothing,
            Err(RecvTimeoutError::Disconnected) => return Waited::Gone,
        };
        while let Ok(more) = self.received.try_recv() {
            changes.extend(more);
        }
        sorted(changes, &self.root)
    }
}

/// What the changes say, from the folder watched: the folder gone, changes lost, or the paths that changed, each once.
/// A folder whose times alone changed is left out, as on the other systems: the files created or deleted in it are
/// said on their own.
fn sorted(changes: Vec<Change>, root: &Path) -> Waited {
    let mut paths = Vec::new();
    let mut seen = BTreeSet::new();
    let mut has_overflowed = false;
    for change in changes {
        if change.flags & fs::kFSEventStreamEventFlagRootChanged != 0 {
            return Waited::Gone;
        }
        if change.flags & LOST != 0 {
            has_overflowed = true;
            continue;
        }
        if change.flags & fs::kFSEventStreamEventFlagHistoryDone != 0 {
            continue;
        }
        let Ok(relative) = change.path.strip_prefix(root) else {
            continue;
        };
        let is_folder = change.flags & fs::kFSEventStreamEventFlagItemIsDir != 0;
        if relative.as_os_str().is_empty() || (is_folder && change.flags & NAMED == 0) {
            continue;
        }
        if seen.insert(relative.to_path_buf()) {
            paths.push(relative.to_path_buf());
        }
    }
    if has_overflowed {
        Waited::Overflowed
    } else if paths.is_empty() {
        Waited::Nothing
    } else {
        Waited::Changes(paths)
    }
}

/// Counts the stream as one more owner of the deliveries.
unsafe extern "C" fn retain(info: *const c_void) -> *const c_void {
    // SAFETY: `info` is the pointer of an `Arc` of the deliveries, alive while one of its owners is, as this one is.
    unsafe { Arc::increment_strong_count(info.cast::<Deliveries>()) };
    info
}

/// The stream lets go of the deliveries.
unsafe extern "C" fn release(info: *const c_void) {
    // SAFETY: `info` is the pointer of an `Arc` of the deliveries that `retain` counted for the stream.
    unsafe { Arc::decrement_strong_count(info.cast::<Deliveries>()) };
}

/// What the stream calls on its queue: the changes are read out of what the system lent and sent to the thread that
/// waits. Nothing here can unwind into the system.
unsafe extern "C" fn told(
    _stream: fs::ConstFSEventStreamRef,
    info: *mut c_void,
    count: usize,
    paths: *mut c_void,
    flags: *const fs::FSEventStreamEventFlags,
    _ids: *const fs::FSEventStreamEventId,
) {
    let paths = paths.cast_const().cast::<*const core::ffi::c_char>();
    if info.is_null() || paths.is_null() || flags.is_null() || count == 0 {
        return;
    }
    // SAFETY: `info` is the `Arc` of the deliveries the stream retained, alive for the call; `paths` and `flags` hold
    // `count` items each, lent by the system for the call, and every path is a C string.
    let (deliveries, paths, flags) = unsafe {
        (
            &*info.cast_const().cast::<Deliveries>(),
            core::slice::from_raw_parts(paths, count),
            core::slice::from_raw_parts(flags, count),
        )
    };
    let changes = paths
        .iter()
        .zip(flags)
        .filter(|(path, _)| !path.is_null())
        .map(|(path, flags)| Change {
            // SAFETY: the path is a C string the system lent for the call, copied before it returns.
            path: PathBuf::from(OsStr::from_bytes(unsafe { CStr::from_ptr(*path) }.to_bytes())),
            flags: *flags,
        })
        .collect();
    let _sent = deliveries.send(changes);
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{Change, LOST, sorted};
    use crate::macos::core_services as fs;
    use crate::watch::Waited;

    fn change(path: &str, flags: u32) -> Change {
        Change {
            path: PathBuf::from(path),
            flags,
        }
    }

    #[test]
    fn the_paths_are_said_from_the_folder_once_each_and_the_times_of_folders_are_left_out() {
        let root = Path::new("/private/var/repo");
        let changes = vec![
            change("/private/var/repo/src/a.rs", fs::kFSEventStreamEventFlagItemCreated),
            change("/private/var/repo/src/a.rs", 0),
            change("/private/var/repo/src", fs::kFSEventStreamEventFlagItemIsDir),
            change(
                "/private/var/repo/new",
                fs::kFSEventStreamEventFlagItemIsDir | fs::kFSEventStreamEventFlagItemCreated,
            ),
            change("/private/var/repo", fs::kFSEventStreamEventFlagItemIsDir),
            change("/elsewhere/b.rs", 0),
        ];
        assert_eq!(
            sorted(changes, root),
            Waited::Changes(vec![PathBuf::from("src/a.rs"), PathBuf::from("new")])
        );
    }

    #[test]
    fn changes_lost_or_the_folder_gone_are_said_as_such() {
        let root = Path::new("/repo");
        assert_eq!(
            sorted(vec![change("/repo/a", 0), change("/repo", LOST)], root),
            Waited::Overflowed
        );
        assert_eq!(
            sorted(vec![change("/repo", fs::kFSEventStreamEventFlagRootChanged)], root),
            Waited::Gone
        );
        assert_eq!(sorted(Vec::new(), root), Waited::Nothing);
    }
}
