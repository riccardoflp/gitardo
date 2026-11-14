//! Opening an address in the browser of the system (`docs/05-platform.md`, Shell): `ShellExecuteW` on Windows,
//! `open` on macOS and `xdg-open` elsewhere.

use crate::error::PlatformError;

/// Opens an address in the browser of the system: the page of a pull request on GitHub.
///
/// # Errors
///
/// When the system has nothing to open it with, or refuses.
pub fn open_url(url: &str) -> Result<(), PlatformError> {
    #[cfg(windows)]
    return crate::windows::open_url(url);
    #[cfg(target_os = "macos")]
    return open_with("open", url);
    #[cfg(not(any(windows, target_os = "macos")))]
    return open_with("xdg-open", url);
}

/// Opens an address with a program that hands it to the browser the desktop has for it. The program ends once it
/// handed the address over; a thread waits for it, so that it does not stay behind as a process that ended unseen.
#[cfg(not(windows))]
fn open_with(opener: &'static str, url: &str) -> Result<(), PlatformError> {
    use std::process::{Command, Stdio};
    let mut opening = Command::new(opener)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| PlatformError::system(opener, &error))?;
    let _waiting = std::thread::Builder::new()
        .name(String::from(opener))
        .spawn(move || opening.wait())
        .map_err(|error| PlatformError::system(opener, &error))?;
    Ok(())
}
