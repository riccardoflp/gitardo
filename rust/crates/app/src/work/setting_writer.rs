//! The writer of the settings the window changes, such as the switches of the timeline: a thread that sets each value
//! in its file in turn, keeping the rest of the text, so that the window never waits on the disk. The watcher of the
//! settings files reads the file again afterwards.

use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use json::{Key, Value};

/// A value to set in a settings file.
type Change = (PathBuf, &'static str, Value);

/// The thread that writes the settings the window changes.
pub struct SettingWriter {
    /// Where the changes go; none once the writer is closing.
    sender: Option<Sender<Change>>,
    /// The thread.
    thread: Option<JoinHandle<()>>,
}

impl SettingWriter {
    /// A writer, with its thread.
    #[must_use]
    pub fn new() -> SettingWriter {
        let (sender, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name(String::from("settings writer"))
            .spawn(move || write_all(&receiver))
            .ok();
        SettingWriter {
            sender: Some(sender),
            thread,
        }
    }

    /// Sets a value in a file, after the values given before.
    pub fn write(&self, file: PathBuf, key: &'static str, value: Value) {
        if let Some(sender) = &self.sender {
            let _sent = sender.send((file, key, value));
        }
    }
}

impl Default for SettingWriter {
    fn default() -> SettingWriter {
        SettingWriter::new()
    }
}

impl Drop for SettingWriter {
    /// Writes what waits before the program ends.
    fn drop(&mut self) {
        self.sender = None;
        if let Some(thread) = self.thread.take() {
            let _ended = thread.join();
        }
    }
}

/// What the thread does until the writer is dropped: sets each value in its file. A file that is not there is made;
/// one that is not JSON with comments is left as it is, and the setting stays in force until the window closes.
fn write_all(receiver: &Receiver<Change>) {
    while let Ok((file, key, value)) = receiver.recv() {
        let text = fs::read_to_string(&file).unwrap_or_default();
        let Ok(changed) = json::set_value(&text, &[Key::Member(key.to_owned())], &value) else {
            continue;
        };
        if let Some(folder) = file.parent() {
            let _made = fs::create_dir_all(folder);
        }
        let _written = fs::write(&file, changed);
    }
}
