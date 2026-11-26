//! Watching a folder and everything under it with `ReadDirectoryChangesW`, in overlapped mode, so that the thread that
//! waits can also look, a few times a second, whether it is to stop.

use core::ffi::c_void;
use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use super::kernel32::{self, HANDLE, OVERLAPPED};
use crate::error::SystemError;
use crate::watch::Waited;

/// How many 32-bit words the buffer of the changes holds: 64 KiB, the most the system takes for a folder of the network.
const BUFFER_WORDS: usize = 16 * 1024;

/// The changes asked for: names created, deleted and renamed, and files written. Not the times things were read.
const WATCHED: u32 = kernel32::FILE_NOTIFY_CHANGE_FILE_NAME
    | kernel32::FILE_NOTIFY_CHANGE_DIR_NAME
    | kernel32::FILE_NOTIFY_CHANGE_SIZE
    | kernel32::FILE_NOTIFY_CHANGE_LAST_WRITE
    | kernel32::FILE_NOTIFY_CHANGE_CREATION;

/// A handle of the system, closed when dropped.
struct Handle(HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: the handle was opened by the system for this value, which owns it and closes it once.
        let _closed = unsafe { kernel32::CloseHandle(self.0) };
    }
}

/// A folder watched, with the call to the system under way.
pub(crate) struct DirectoryWatch {
    /// The folder, opened to list it.
    directory: Handle,
    /// The event the system sets when changes come, held to be closed with the watch.
    #[allow(
        dead_code,
        reason = "the event is only held, so that it is closed when the watch is dropped"
    )]
    event: Handle,
    /// What the call under way shares with the system: boxed, so that it stays where the system writes it.
    overlapped: Box<OVERLAPPED>,
    /// Where the system writes the changes: boxed for the same reason, and in words, which aligns it as it must be.
    buffer: Box<[u32]>,
    /// Whether a call is under way.
    is_pending: bool,
}

// SAFETY: the handles of a folder and of an event may be used from any thread, and the overlapped and the buffer are
// owned by the value, which one thread at a time uses: it is opened on one thread and moved to the one that waits.
unsafe impl Send for DirectoryWatch {}

impl DirectoryWatch {
    /// Opens a folder to watch it and everything under it, and asks for its first changes.
    pub(crate) fn open(folder: &Path) -> Result<DirectoryWatch, SystemError> {
        let name: Vec<u16> = folder.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: `name` ends with a NUL and lives for the call; the other arguments are flags and null pointers the
        // documentation allows.
        let directory = unsafe {
            kernel32::CreateFileW(
                name.as_ptr(),
                kernel32::FILE_LIST_DIRECTORY,
                kernel32::FILE_SHARE_READ | kernel32::FILE_SHARE_WRITE | kernel32::FILE_SHARE_DELETE,
                std::ptr::null_mut(),
                kernel32::OPEN_EXISTING,
                kernel32::FILE_FLAG_BACKUP_SEMANTICS | kernel32::FILE_FLAG_OVERLAPPED,
                std::ptr::null_mut(),
            )
        };
        if directory.is_null() || directory as isize == -1 {
            return Err(SystemError::system("CreateFileW", &last_error()));
        }
        let directory = Handle(directory);
        // SAFETY: a manual-reset event, not set, with no name and no attributes.
        let event = unsafe { kernel32::CreateEventW(std::ptr::null_mut(), 1, 0, std::ptr::null()) };
        if event.is_null() {
            return Err(SystemError::system("CreateEventW", &last_error()));
        }
        let mut watch = DirectoryWatch {
            directory,
            overlapped: Box::new(OVERLAPPED {
                Internal: 0,
                InternalHigh: 0,
                Offset: 0,
                OffsetHigh: 0,
                hEvent: event,
            }),
            event: Handle(event),
            buffer: vec![0; BUFFER_WORDS].into_boxed_slice(),
            is_pending: false,
        };
        // The first changes are asked for at once, so that what changes from now on is said, before the thread that
        // waits for them runs; the overlapped and the buffer are boxed, and stay where they are when the value moves.
        if !watch.ask() {
            return Err(SystemError::system("ReadDirectoryChangesW", &last_error()));
        }
        Ok(watch)
    }

    /// Waits at most some milliseconds for the changes of the folder.
    pub(crate) fn wait(&mut self, milliseconds: u32) -> Waited {
        if !self.is_pending && !self.ask() {
            return Waited::Gone;
        }
        let mut bytes: u32 = 0;
        // SAFETY: the handle, the box of the overlapped and `bytes` are alive; the call under way was started with
        // this overlapped, which stays where it is until it completes.
        let done = unsafe {
            kernel32::GetOverlappedResultEx(
                self.directory.0,
                &raw mut *self.overlapped,
                &raw mut bytes,
                milliseconds,
                0,
            )
        };
        if done == 0 {
            // SAFETY: the call takes nothing.
            return match unsafe { kernel32::GetLastError() } {
                kernel32::WAIT_TIMEOUT | kernel32::ERROR_IO_INCOMPLETE => Waited::Nothing,
                kernel32::ERROR_NOTIFY_ENUM_DIR => {
                    self.is_pending = false;
                    Waited::Overflowed
                }
                _ => {
                    self.is_pending = false;
                    Waited::Gone
                }
            };
        }
        self.is_pending = false;
        let length = usize::try_from(bytes).unwrap_or_default();
        if length == 0 {
            return Waited::Overflowed;
        }
        let bytes: Vec<u8> = self
            .buffer
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .take(length)
            .collect();
        Waited::Changes(parse_changes(&bytes))
    }

    /// Asks the system for the next changes; false when it refuses.
    fn ask(&mut self) -> bool {
        let words = u32::try_from(self.buffer.len() * 4).unwrap_or(u32::MAX);
        // SAFETY: the handle, the buffer and the overlapped are alive and stay where they are until the call completes
        // or is cancelled in `drop`; the event of the overlapped is the one this value owns.
        let asked = unsafe {
            kernel32::ReadDirectoryChangesW(
                self.directory.0,
                self.buffer.as_mut_ptr().cast::<c_void>(),
                words,
                1,
                WATCHED,
                std::ptr::null_mut(),
                &raw mut *self.overlapped,
                std::ptr::null_mut(),
            )
        };
        self.is_pending = asked != 0;
        self.is_pending
    }
}

impl Drop for DirectoryWatch {
    /// Cancels the call under way and waits for it to end, so that the system no longer writes in the buffer.
    fn drop(&mut self) {
        if self.is_pending {
            let mut bytes: u32 = 0;
            // SAFETY: the call under way uses this handle and this overlapped, both alive.
            let _cancelled = unsafe { kernel32::CancelIoEx(self.directory.0, &raw mut *self.overlapped) };
            // SAFETY: as above; waiting makes sure the system is done with the buffer before it is freed.
            let _ended = unsafe {
                kernel32::GetOverlappedResult(self.directory.0, &raw mut *self.overlapped, &raw mut bytes, 1)
            };
        }
    }
}

/// The paths of the records `FILE_NOTIFY_INFORMATION` the system wrote: each its offset to the next, what happened,
/// the length of its name in bytes and the name, in UTF-16.
fn parse_changes(bytes: &[u8]) -> Vec<PathBuf> {
    let word = |at: usize| {
        bytes
            .get(at..at + 4)
            .and_then(|slice| slice.try_into().ok())
            .map_or(0, u32::from_le_bytes)
    };
    let mut paths = Vec::new();
    let mut offset = 0;
    loop {
        let next = usize::try_from(word(offset)).unwrap_or_default();
        let length = usize::try_from(word(offset + 8)).unwrap_or_default();
        let name: Vec<u16> = bytes
            .get(offset + 12..offset + 12 + length)
            .unwrap_or_default()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes(*pair))
            .collect();
        if !name.is_empty() {
            paths.push(PathBuf::from(OsString::from_wide(&name)));
        }
        if next == 0 || offset + next >= bytes.len() {
            return paths;
        }
        offset += next;
    }
}

/// The last error of the system, in its words.
fn last_error() -> std::io::Error {
    std::io::Error::last_os_error()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::parse_changes;

    fn record(next: u32, name: &str) -> Vec<u8> {
        let name: Vec<u8> = name.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let mut bytes = Vec::new();
        bytes.extend(next.to_le_bytes());
        bytes.extend(3_u32.to_le_bytes());
        bytes.extend(u32::try_from(name.len()).unwrap_or_default().to_le_bytes());
        bytes.extend(name);
        while bytes.len() % 4 != 0 {
            bytes.push(0);
        }
        bytes
    }

    #[test]
    fn reads_every_record_of_the_buffer() {
        let first = record(0, "src\\a.rs");
        let mut bytes = record(u32::try_from(first.len()).unwrap_or_default(), "src\\a.rs");
        bytes.extend(record(0, ".git\\refs\\heads\\main"));
        assert_eq!(
            parse_changes(&bytes),
            [PathBuf::from("src\\a.rs"), PathBuf::from(".git\\refs\\heads\\main")]
        );
    }
}
