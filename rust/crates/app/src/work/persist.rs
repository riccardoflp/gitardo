//! The writer of what is kept between runs: a thread that writes the file, so that the window never waits on the disk.
//! When several versions wait, only the latest is written.

use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::persisted::Persisted;

/// The thread that writes the file of what is kept between runs.
pub struct PersistedWriter {
    /// Where the versions to write go; none once the writer is closing.
    sender: Option<Sender<Persisted>>,
    /// The thread.
    thread: Option<JoinHandle<()>>,
}

impl PersistedWriter {
    /// A writer of a file, whose folder is made when it is not there.
    #[must_use]
    pub fn new(file: PathBuf) -> PersistedWriter {
        let (sender, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name(String::from("persisted state writer"))
            .spawn(move || write_all(&receiver, &file))
            .ok();
        PersistedWriter {
            sender: Some(sender),
            thread,
        }
    }

    /// Writes a version, after those given before.
    pub fn write(&self, persisted: Persisted) {
        if let Some(sender) = &self.sender {
            let _sent = sender.send(persisted);
        }
    }
}

impl Drop for PersistedWriter {
    /// Writes what waits before the program ends.
    fn drop(&mut self) {
        self.sender = None;
        if let Some(thread) = self.thread.take() {
            let _ended = thread.join();
        }
    }
}

/// What the thread does until the writer is dropped: writes the latest version of those waiting.
fn write_all(receiver: &Receiver<Persisted>, file: &PathBuf) {
    while let Ok(first) = receiver.recv() {
        let latest = receiver.try_iter().last().unwrap_or(first);
        if let Some(folder) = file.parent() {
            let _made = fs::create_dir_all(folder);
        }
        // A file that cannot be written leaves the last one: the next run starts from what it holds.
        let _written = fs::write(file, latest.to_text());
    }
}
