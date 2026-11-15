//! The JSON of the messages of the journal, from stage 3: each read back as it was written, and each record of the
//! extension made into the engine's.

#![cfg(test)]

use model::contract::UndoRecord;
use protocol::journal::{
    JournalEntryJson, JournalLastResult, JournalRecordParams, JournalRecordResult, UndoRecordJson, head_of,
};
use protocol::state::HeadJson;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;

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
fn every_message_of_the_journal_is_read_back_as_it_was_written() {
    let records = [
        UndoRecordJson::Reflog,
        UndoRecordJson::NotUndoable {
            reason: String::from("A push changes the remote."),
        },
        UndoRecordJson::BranchCreated {
            name: String::from("spike"),
        },
        UndoRecordJson::BranchDeleted {
            name: String::from("old"),
            commit: "b".repeat(40),
        },
        UndoRecordJson::StashDropped {
            message: String::from("wip"),
            commit: "c".repeat(40),
        },
        UndoRecordJson::ConflictsResolved {
            files: vec![String::from("a.txt")],
        },
        UndoRecordJson::SquashMerge,
        UndoRecordJson::Series {
            head_before: "d".repeat(40),
        },
        UndoRecordJson::SplitLast { commit: "e".repeat(40) },
    ];
    for record in records {
        round_trip(&record);
        assert!(UndoRecord::try_from(record).is_ok());
    }
    round_trip(&JournalRecordParams {
        repository: String::from("/work/app"),
        title: String::from("Delete branch old"),
        record: None,
        before: head(),
        after: head(),
    });
    round_trip(&JournalRecordResult { number: Some(3) });
    round_trip(&JournalLastResult {
        entry: Some(JournalEntryJson {
            number: 3,
            title: String::from("Delete branch old"),
            undoable: true,
            head: head(),
        }),
    });
}

#[test]
fn a_record_is_written_with_its_kind_and_a_commit_that_is_not_one_is_refused() {
    let series: UndoRecordJson =
        serde_json::from_value(json!({ "kind": "series", "headBefore": "a".repeat(40) })).unwrap();
    assert!(matches!(UndoRecord::try_from(series), Ok(UndoRecord::Series { .. })));
    let bad = UndoRecordJson::BranchDeleted {
        name: String::from("old"),
        commit: String::from("nope"),
    };
    assert!(UndoRecord::try_from(bad).is_err());
    assert!(
        head_of(&HeadJson {
            commit: Some(String::from("x")),
            branch: None
        })
        .is_err()
    );
}
