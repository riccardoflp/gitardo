//! Opening an address in the browser of the system, through `ShellExecuteW`.

use super::shell32;
use crate::error::PlatformError;

/// The value above which `ShellExecuteW` says it succeeded.
const LAST_FAILURE: usize = 32;

/// A text in UTF-16 ended by a NUL.
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

/// Opens an address with the program the system has for it: the browser for `https`.
pub(crate) fn open_url(url: &str) -> Result<(), PlatformError> {
    let operation = wide("open");
    let file = wide(url);
    // SAFETY: both texts are ended by a NUL and live through the call; no window, parameters or folder are given, which
    // the call takes as null.
    let result = unsafe {
        shell32::ShellExecuteW(
            core::ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            core::ptr::null(),
            core::ptr::null(),
            shell32::SW_SHOWNORMAL,
        )
    };
    if result.addr() > LAST_FAILURE {
        Ok(())
    } else {
        Err(PlatformError::system(
            "ShellExecuteW",
            &format!("it answered {}", result.addr()),
        ))
    }
}
