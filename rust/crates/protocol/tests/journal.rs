//! The JSON of the message of the journal, from stage 3, read back as it was written.

#![cfg(test)]

use protocol::journal::{JournalEntryJson, JournalLastResult};
use protocol::state::HeadJson;
use serde::Serialize;
use serde::de::DeserializeOwned;

fn round_trip<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: &T) {
    let text = serde_json::to_string(value).unwrap();
    let read: T = serde_json::from_str(&text).unwrap();
    assert_eq!(&read, value, "{text}");
}

fn head() -> HeadJson {
    HeadJson {
        commit: Some("a".repeat(40)),
        branch: Some(String::from("refs/heads/main")),
    }
}

#[test]
fn the_newest_entry_of_the_journal_is_read_back_as_it_was_written() {
    round_trip(&JournalLastResult {
        entry: Some(JournalEntryJson {
            number: 3,
            title: String::from("Delete branch old"),
            undoable: true,
            head: head(),
        }),
    });
    round_trip(&JournalLastResult { entry: None });
}
