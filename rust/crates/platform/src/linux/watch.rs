//! Watching a folder and everything under it with inotify. Inotify watches one folder at a time, so every folder under
//! the one watched gets a watch of its own, and a folder created or moved in gets one as it comes. `poll` waits with a
//! timeout, so that the thread that waits can also look, a few times a second, whether it is to stop.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{CString, OsString};
use std::fs::File;
use std::io::{ErrorKind, Read};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use super::libc;
use crate::error::PlatformError;
use crate::watch::Waited;

/// How many bytes are read at a time: 64 KiB, room for a thousand changes with long names.
const BUFFER_BYTES: usize = 64 * 1024;

/// The size of a record before its name: the watch, what happened, the cookie of a rename and the length of the name.
const HEADER_BYTES: usize = 16;

/// The changes asked for: names created, deleted and moved, files written, and the folder itself gone. Not the times
/// things were read, nor their attributes. Links to folders are not followed, so that a folder linked from elsewhere,
/// such as a shared `node_modules`, is not watched through the link.
const WATCHED: u32 = libc::IN_MODIFY
    | libc::IN_CLOSE_WRITE
    | libc::IN_CREATE
    | libc::IN_DELETE
    | libc::IN_MOVED_FROM
    | libc::IN_MOVED_TO
    | libc::IN_DELETE_SELF
    | libc::IN_MOVE_SELF
    | libc::IN_ONLYDIR
    | libc::IN_DONT_FOLLOW
    | libc::IN_EXCL_UNLINK;

/// One record inotify wrote: the watch it came from, what happened, and the name under that watch's folder.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Record {
    watch: i32,
    mask: u32,
    name: Option<OsString>,
}

/// A folder watched, with a watch for each folder under it.
pub(crate) struct DirectoryWatch {
    /// The inotify instance, read without waiting; closed when the watch is dropped.
    file: File,
    /// The watch of the folder itself.
    root: i32,
    /// The folder of each watch, from the folder watched: empty for the folder itself.
    folders: BTreeMap<i32, PathBuf>,
    /// The folder watched, whole.
    folder: PathBuf,
    /// Where the records are read into.
    buffer: Box<[u8]>,
}

impl DirectoryWatch {
    /// Opens a folder to watch it and everything under it. What changes from now on is said, also under the folders
    /// it has now.
    pub(crate) fn open(folder: &Path) -> Result<DirectoryWatch, PlatformError> {
        // SAFETY: the call takes only flags.
        let descriptor = unsafe { libc::inotify_init1(libc::IN_NONBLOCK | libc::IN_CLOEXEC) };
        if descriptor < 0 {
            return Err(PlatformError::system("inotify_init1", &std::io::Error::last_os_error()));
        }
        // SAFETY: the descriptor was just opened by the system for this value, which alone owns and closes it.
        let file = File::from(unsafe { OwnedFd::from_raw_fd(descriptor) });
        let mut watch = DirectoryWatch {
            file,
            root: -1,
            folders: BTreeMap::new(),
            folder: folder.to_path_buf(),
            buffer: vec![0; BUFFER_BYTES].into_boxed_slice(),
        };
        watch.root = watch
            .add(Path::new(""))
            .map_err(|error| PlatformError::system("inotify_add_watch", &error))?;
        let mut found = Vec::new();
        watch.add_under(Path::new(""), &mut found);
        Ok(watch)
    }

    /// Waits at most some milliseconds for the changes of the folder.
    pub(crate) fn wait(&mut self, milliseconds: u32) -> Waited {
        let mut asked = libc::pollfd {
            fd: self.file.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let timeout = i32::try_from(milliseconds).unwrap_or(i32::MAX);
        // SAFETY: `asked` is one `pollfd`, alive for the call, and the count says one.
        let ready = unsafe { libc::poll(&raw mut asked, 1, timeout) };
        if ready < 0 {
            let error = std::io::Error::last_os_error();
            return if error.kind() == ErrorKind::Interrupted {
                Waited::Nothing
            } else {
                Waited::Gone
            };
        }
        if ready == 0 {
            return Waited::Nothing;
        }
        let length = match self.file.read(&mut self.buffer) {
            Ok(0) => return Waited::Gone,
            Ok(length) => length,
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted) => {
                return Waited::Nothing;
            }
            Err(_) => return Waited::Gone,
        };
        let records = parse_records(self.buffer.get(..length).unwrap_or_default());
        self.take(records)
    }

    /// What the records say changed, with the watches of the folders that came and went brought up to date.
    fn take(&mut self, records: Vec<Record>) -> Waited {
        let mut changed = Vec::new();
        let mut has_overflowed = false;
        for record in records {
            if record.mask & libc::IN_Q_OVERFLOW != 0 {
                has_overflowed = true;
                continue;
            }
            let gone = libc::IN_DELETE_SELF | libc::IN_MOVE_SELF | libc::IN_IGNORED;
            if record.watch == self.root && record.mask & gone != 0 {
                return Waited::Gone;
            }
            if record.mask & libc::IN_IGNORED != 0 {
                let _forgotten = self.folders.remove(&record.watch);
                continue;
            }
            let (Some(folder), Some(name)) = (self.folders.get(&record.watch), record.name) else {
                continue;
            };
            let path = folder.join(name);
            if record.mask & libc::IN_ISDIR != 0 {
                if record.mask & (libc::IN_MOVED_FROM | libc::IN_DELETE) != 0 {
                    self.forget_under(&path);
                }
                if record.mask & (libc::IN_CREATE | libc::IN_MOVED_TO) != 0 && self.add(&path).is_ok() {
                    self.add_under(&path, &mut changed);
                }
            }
            changed.push(path);
        }
        if has_overflowed {
            // Folders made while the records were lost have no watch yet: every folder is asked for again, which
            // gives the folders already watched the watch they have.
            let mut found = Vec::new();
            self.add_under(Path::new(""), &mut found);
            return Waited::Overflowed;
        }
        let mut seen = BTreeSet::new();
        changed.retain(|path| seen.insert(path.clone()));
        if changed.is_empty() {
            Waited::Nothing
        } else {
            Waited::Changes(changed)
        }
    }

    /// Adds a watch for a folder, given from the folder watched.
    fn add(&mut self, relative: &Path) -> std::io::Result<i32> {
        let whole = self.folder.join(relative);
        let name = CString::new(whole.as_os_str().as_bytes()).map_err(std::io::Error::other)?;
        // SAFETY: the descriptor is the inotify instance this value owns, and the name ends with a NUL and lives for
        // the call.
        let watch = unsafe { libc::inotify_add_watch(self.file.as_raw_fd(), name.as_ptr(), WATCHED) };
        if watch < 0 {
            return Err(std::io::Error::last_os_error());
        }
        let _replaced = self.folders.insert(watch, relative.to_path_buf());
        Ok(watch)
    }

    /// Adds a watch for every folder under one already watched, and puts in `found` every path under it: what was
    /// made in a folder before its watch was added is said this way. A folder that cannot be watched, as when the
    /// system's limit of watches is reached, is left out.
    fn add_under(&mut self, relative: &Path, found: &mut Vec<PathBuf>) {
        let mut waiting = vec![relative.to_path_buf()];
        while let Some(folder) = waiting.pop() {
            let Ok(entries) = std::fs::read_dir(self.folder.join(&folder)) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = folder.join(entry.file_name());
                // The type of the entry itself: a link to a folder is not a folder here, and is not followed.
                if entry.file_type().is_ok_and(|kind| kind.is_dir()) && self.add(&path).is_ok() {
                    waiting.push(path.clone());
                }
                found.push(path);
            }
        }
    }

    /// Takes away the watches of a folder and of every folder under it: they were moved away or deleted.
    fn forget_under(&mut self, relative: &Path) {
        let watches: Vec<i32> = self
            .folders
            .iter()
            .filter(|(watch, folder)| **watch != self.root && folder.starts_with(relative))
            .map(|(watch, _)| *watch)
            .collect();
        for watch in watches {
            let _forgotten = self.folders.remove(&watch);
            // SAFETY: the descriptor is the inotify instance this value owns; a watch the system already took away
            // only makes the call fail.
            let _removed = unsafe { libc::inotify_rm_watch(self.file.as_raw_fd(), watch) };
        }
    }
}

/// The records `inotify_event` the system wrote: each its watch, what happened, the cookie of a rename, the length of
/// its name in bytes, and the name, ended and padded by NULs. The numbers are in the order of the machine.
fn parse_records(bytes: &[u8]) -> Vec<Record> {
    let word = |at: usize| {
        bytes
            .get(at..at + 4)
            .and_then(|slice| slice.try_into().ok())
            .map_or(0, u32::from_ne_bytes)
    };
    let mut records = Vec::new();
    let mut offset = 0;
    while offset + HEADER_BYTES <= bytes.len() {
        let length = usize::try_from(word(offset + 12)).unwrap_or_default();
        let name: Vec<u8> = bytes
            .get(offset + HEADER_BYTES..offset + HEADER_BYTES + length)
            .unwrap_or_default()
            .iter()
            .copied()
            .take_while(|byte| *byte != 0)
            .collect();
        records.push(Record {
            watch: word(offset).cast_signed(),
            mask: word(offset + 4),
            name: (!name.is_empty()).then(|| OsString::from_vec(name)),
        });
        offset += HEADER_BYTES + length;
    }
    records
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::{Record, parse_records};
    use crate::linux::libc;

    fn record(watch: i32, mask: u32, name: &str) -> Vec<u8> {
        let mut padded = name.as_bytes().to_vec();
        if !padded.is_empty() {
            padded.push(0);
            while !padded.len().is_multiple_of(16) {
                padded.push(0);
            }
        }
        let mut bytes = Vec::new();
        bytes.extend(watch.to_ne_bytes());
        bytes.extend(mask.to_ne_bytes());
        bytes.extend(0_u32.to_ne_bytes());
        bytes.extend(u32::try_from(padded.len()).unwrap_or_default().to_ne_bytes());
        bytes.extend(padded);
        bytes
    }

    #[test]
    fn reads_every_record_of_the_buffer() {
        let mut bytes = record(1, libc::IN_CLOSE_WRITE, "a.rs");
        bytes.extend(record(2, libc::IN_CREATE | libc::IN_ISDIR, "a folder with a long name"));
        bytes.extend(record(1, libc::IN_IGNORED, ""));
        assert_eq!(
            parse_records(&bytes),
            [
                Record {
                    watch: 1,
                    mask: libc::IN_CLOSE_WRITE,
                    name: Some(OsString::from("a.rs")),
                },
                Record {
                    watch: 2,
                    mask: libc::IN_CREATE | libc::IN_ISDIR,
                    name: Some(OsString::from("a folder with a long name")),
                },
                Record {
                    watch: 1,
                    mask: libc::IN_IGNORED,
                    name: None,
                },
            ]
        );
        assert_eq!(parse_records(&bytes[..10]), []);
    }
}
