//! Watching a folder: a file written under it is said, with its whole path, and dropping the watcher stops it.

#![cfg(any(windows, target_os = "linux"))]
#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]

use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use platform::{WatchDeliver, WatchEvent, Watcher};

#[test]
fn a_file_written_under_the_folder_is_said_with_its_whole_path() {
    let folder = std::env::temp_dir().join(format!("gitardo-watch-{}", std::process::id()));
    std::fs::create_dir_all(folder.join("src")).unwrap();
    let (sender, receiver) = mpsc::channel();
    let sender = Mutex::new(sender);
    let deliver: WatchDeliver = Arc::new(move |events| {
        let _sent = sender.lock().unwrap().send(events);
    });
    let watcher = Watcher::new(std::slice::from_ref(&folder), &deliver).unwrap();
    std::fs::write(folder.join("src").join("a.rs"), "fn main() {}\n").unwrap();
    let wanted = WatchEvent::Changed(folder.join("src").join("a.rs"));
    let mut seen: Vec<WatchEvent> = Vec::new();
    while !seen.contains(&wanted) {
        seen.extend(receiver.recv_timeout(Duration::from_secs(5)).unwrap());
    }
    drop(watcher);
    std::fs::remove_dir_all(&folder).unwrap();
    let paths: Vec<PathBuf> = seen
        .into_iter()
        .filter_map(|event| match event {
            WatchEvent::Changed(path) => Some(path),
            WatchEvent::Overflowed(_) | WatchEvent::Gone(_) => None,
        })
        .collect();
    assert!(paths.iter().all(|path| path.starts_with(&folder)), "{paths:?}");
}
