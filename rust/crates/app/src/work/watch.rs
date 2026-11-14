//! The watcher of the settings files: a thread that looks at the user's file and the repository's a few times a second,
//! and reads the settings again when either changed, so that a change applies without restarting (`settings.read`).
//!
//! It compares the time each file was changed and its length, which costs two small calls to the system each time.
//! The watcher of the repository's files (`repo.watch-changes`, `docs/desktop/02-architecture.md`, Threads) replaces
//! it once it exists.

use std::fs;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

use super::Deliver;
use crate::settings::{SettingsFiles, read_files};
use crate::store::Message;

/// How often the files are looked at: a change applies within this, and the time a read takes.
pub const WATCH_INTERVAL: Duration = Duration::from_millis(250);

/// What a file was like when last looked at: when it was changed and its length; none when it is not there.
type Stamp = Option<(SystemTime, u64)>;

/// The thread that watches the settings files of a window.
pub struct SettingsWatcher {
    /// Where the files to watch go; none once the watcher is closing.
    sender: Option<Sender<SettingsFiles>>,
    /// The thread.
    thread: Option<JoinHandle<()>>,
}

impl SettingsWatcher {
    /// A watcher that watches nothing yet and gives the settings it reads to `deliver`.
    #[must_use]
    pub fn new(deliver: Deliver) -> SettingsWatcher {
        let (sender, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name(String::from("settings watcher"))
            .spawn(move || watch(&receiver, &deliver))
            .ok();
        SettingsWatcher {
            sender: Some(sender),
            thread,
        }
    }

    /// Watches these files in place of those watched before; the settings are read from them at once.
    pub fn watch(&self, files: SettingsFiles) {
        if let Some(sender) = &self.sender {
            let _sent = sender.send(files);
        }
    }
}

impl Drop for SettingsWatcher {
    fn drop(&mut self) {
        self.sender = None;
        if let Some(thread) = self.thread.take() {
            let _ended = thread.join();
        }
    }
}

/// What the thread does until the watcher is dropped.
fn watch(receiver: &Receiver<SettingsFiles>, deliver: &Deliver) {
    let mut watched: Option<(SettingsFiles, Vec<Stamp>)> = None;
    loop {
        match receiver.recv_timeout(WATCH_INTERVAL) {
            Ok(files) => {
                let stamps = stamps(&files);
                deliver(Message::SettingsRead(Box::new(read_files(&files))));
                watched = Some((files, stamps));
            }
            Err(RecvTimeoutError::Timeout) => {
                if let Some((files, seen)) = watched.as_mut() {
                    let now = stamps(files);
                    if now != *seen {
                        *seen = now;
                        deliver(Message::SettingsRead(Box::new(read_files(files))));
                    }
                }
            }
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

/// What the files are like now.
fn stamps(files: &SettingsFiles) -> Vec<Stamp> {
    [&files.user, &files.repository]
        .into_iter()
        .map(|file| {
            let metadata = fs::metadata(file.as_ref()?).ok()?;
            Some((metadata.modified().ok()?, metadata.len()))
        })
        .collect()
}
