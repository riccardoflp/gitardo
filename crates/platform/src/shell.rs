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
    #[cfg(target_os = "linux")]
    return crate::linux::open_url(url);
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
        return std::process::Command::new(opener)
            .arg(url)
            .spawn()
            .map(drop)
            .map_err(|error| PlatformError::system("open", &error));
    }
}
