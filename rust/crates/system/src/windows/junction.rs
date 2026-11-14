//! Junctions: links to folders that Windows makes without the rights a symbolic link needs, through a reparse point
//! of the kind `IO_REPARSE_TAG_MOUNT_POINT` set on an empty folder.

use core::ffi::c_void;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf, Prefix};

use super::kernel32;
use crate::error::SystemError;

/// The bytes of the header of a reparse point before its data: its tag, the length of its data and a reserved word.
const HEADER_BYTES: usize = 8;
/// The bytes of the places and lengths of the two names of a mount point, before the names.
const NAMES_HEADER_BYTES: usize = 8;
/// What is said when the path of the folder linked to is too long for a junction.
const TOO_LONG: &str = "make a junction to a folder whose path is so long";

/// A handle of the system, closed when dropped.
struct Handle(kernel32::HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: the handle was opened by the system for this value, which owns it and closes it once.
        let _closed = unsafe { kernel32::CloseHandle(self.0) };
    }
}

/// Makes `link` a junction to the folder `target`: an empty folder is made at `link`, and the junction set on it. The
/// target is made absolute; the folder made is taken away again when the system refuses the junction.
pub(crate) fn make_junction(link: &Path, target: &Path) -> Result<(), SystemError> {
    let target =
        std::path::absolute(target).map_err(|error| SystemError::system("the folder of a junction", &error))?;
    let buffer = reparse_buffer(&target)?;
    fs::create_dir(link).map_err(|error| SystemError::system("a folder for a junction", &error))?;
    let set = set_reparse_point(link, buffer);
    if set.is_err() {
        // The empty folder is taken away so that nothing is left where the junction was to be; when it cannot be, the
        // error of the junction is the one said.
        let _removed = fs::remove_dir(link);
    }
    set
}

/// The data of the reparse point of a junction to `target`, as `REPARSE_DATA_BUFFER` lays out a mount point: the tag,
/// the length of the data, the place and length of the name the system follows (`\??\C:\folder`) and of the name it
/// prints (`C:\folder`), then both names in UTF-16, each ended by a NUL. It is in 16-bit words, aligned as the system
/// asks.
fn reparse_buffer(target: &Path) -> Result<Vec<u16>, SystemError> {
    let printed: Vec<u16> = plain_path(target).as_os_str().encode_wide().collect();
    let substitute: Vec<u16> = "\\??\\".encode_utf16().chain(printed.iter().copied()).collect();
    let too_long = || SystemError::Unsupported { what: TOO_LONG };
    let substitute_bytes = u16::try_from(substitute.len() * 2).map_err(|_| too_long())?;
    let printed_bytes = u16::try_from(printed.len() * 2).map_err(|_| too_long())?;
    let data_bytes = NAMES_HEADER_BYTES + usize::from(substitute_bytes) + 2 + usize::from(printed_bytes) + 2;
    let data_bytes = u16::try_from(data_bytes).map_err(|_| too_long())?;
    let [tag_low, tag_high] = [
        kernel32::IO_REPARSE_TAG_MOUNT_POINT & 0xFFFF,
        kernel32::IO_REPARSE_TAG_MOUNT_POINT >> 16,
    ]
    .map(|half| u16::try_from(half).unwrap_or_default());
    let mut words: Vec<u16> = Vec::with_capacity(HEADER_BYTES / 2 + usize::from(data_bytes) / 2);
    // The tag in two words, the low one first, as a 32-bit number is laid out in memory; then the length of the data
    // and the reserved word.
    words.extend([tag_low, tag_high, data_bytes, 0]);
    // The substitute name at 0, then the printed name after it and its NUL.
    words.extend([0, substitute_bytes, substitute_bytes + 2, printed_bytes]);
    words.extend(substitute);
    words.push(0);
    words.extend(printed);
    words.push(0);
    Ok(words)
}

/// A path without the `\\?\` the system puts before a path made absolute or canonical, which a junction does not
/// take: `\\?\C:\folder` is `C:\folder`.
fn plain_path(path: &Path) -> PathBuf {
    let mut components = path.components();
    if let Some(Component::Prefix(prefix)) = components.next()
        && let Prefix::VerbatimDisk(letter) = prefix.kind()
    {
        let mut plain = PathBuf::from(format!("{}:\\", char::from(letter)));
        plain.extend(components.filter(|component| !matches!(component, Component::RootDir)));
        return plain;
    }
    path.to_path_buf()
}

/// Sets the reparse point of a junction on the empty folder `link`.
fn set_reparse_point(link: &Path, mut buffer: Vec<u16>) -> Result<(), SystemError> {
    let name: Vec<u16> = link.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: `name` ends with a NUL and lives for the call; the other arguments are flags and null pointers the
    // documentation allows.
    let handle = unsafe {
        kernel32::CreateFileW(
            name.as_ptr(),
            kernel32::GENERIC_WRITE,
            0,
            std::ptr::null_mut(),
            kernel32::OPEN_EXISTING,
            kernel32::FILE_FLAG_OPEN_REPARSE_POINT | kernel32::FILE_FLAG_BACKUP_SEMANTICS,
            std::ptr::null_mut(),
        )
    };
    if handle.is_null() || handle as isize == -1 {
        return Err(last_error("CreateFileW"));
    }
    let handle = Handle(handle);
    let size = u32::try_from(buffer.len() * 2).map_err(|_| SystemError::Unsupported { what: TOO_LONG })?;
    let mut returned = 0_u32;
    // SAFETY: the handle is open for writing; `buffer` holds `size` bytes laid out as `REPARSE_DATA_BUFFER` and lives
    // for the call, which only reads it; there is no output buffer, and the call is not overlapped.
    let done = unsafe {
        kernel32::DeviceIoControl(
            handle.0,
            kernel32::FSCTL_SET_REPARSE_POINT,
            buffer.as_mut_ptr().cast::<c_void>(),
            size,
            std::ptr::null_mut(),
            0,
            &raw mut returned,
            std::ptr::null_mut(),
        )
    };
    if done == 0 {
        return Err(last_error("DeviceIoControl"));
    }
    Ok(())
}

/// The error the system says it last had on this thread.
fn last_error(call: &'static str) -> SystemError {
    // SAFETY: the call has no arguments and only reads the error of this thread.
    let code = unsafe { kernel32::GetLastError() };
    SystemError::system(call, &std::io::Error::from_raw_os_error(code.cast_signed()))
}
