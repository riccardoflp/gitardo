//! The side of a pseudo-terminal its program writes to (`terminal.view`, `tasks.run`): held open by the window for as
//! long as the terminal is, so that on Linux and macOS what a program printed right before it ended is still read.

use std::fs::File;

use crate::error::SystemError;

/// Opens the side of a pseudo-terminal its program writes to, from the side the window reads. While it is open, the
/// side the window reads never says the program hung up, so what the program printed before it ended stays there to
/// be read, rather than being dropped by a read that finds the program gone.
///
/// # Errors
///
/// When the system does not name that side or refuses to open it; on Windows, whose pseudo-terminals have no such
/// side, always.
pub fn open_program_side(window_side: &File) -> Result<File, SystemError> {
    #[cfg(unix)]
    return crate::unix::open_program_side(window_side);
    #[cfg(not(unix))]
    {
        let _ = window_side;
        Err(SystemError::Unsupported {
            what: "open the side of a pseudo-terminal its program writes to",
        })
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::io::{Read, Write};

    use super::open_program_side;

    #[test]
    fn what_is_written_to_the_program_side_is_read_from_the_window_side() {
        // The side the window reads of a new pseudo-terminal, which `/dev/ptmx` gives on Linux and macOS alike.
        let window = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/ptmx")
            .unwrap();
        crate::unix::unlock_program_side(&window).unwrap();
        let mut program = open_program_side(&window).unwrap();
        program.write_all(b"printed\n").unwrap();
        let mut read = [0_u8; 64];
        let count = (&window).read(&mut read).unwrap();
        assert!(String::from_utf8_lossy(&read[..count]).contains("printed"));
    }
}
