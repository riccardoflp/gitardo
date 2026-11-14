//! Opening an address with `xdg-open`, which hands it to the browser the desktop has for it.

use std::process::{Command, Stdio};
use std::thread;

use crate::error::PlatformError;

/// Opens an address with the program the desktop has for it: the browser for `https`. `xdg-open` ends once it handed
/// the address over; a thread waits for it, so that it does not stay behind as a process that ended unseen.
pub(crate) fn open_url(url: &str) -> Result<(), PlatformError> {
    let mut opener = Command::new("xdg-open")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| PlatformError::system("xdg-open", &error))?;
    let _waiting = thread::Builder::new()
        .name(String::from("xdg-open"))
        .spawn(move || opener.wait())
        .map_err(|error| PlatformError::system("xdg-open", &error))?;
    Ok(())
}
