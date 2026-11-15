//! The JSON of what version 1.7 adds for stage 10, once the extension has no git of its own: the repositories of a
//! workspace with their common git folder, a file of a comparison and a change of a diff as targets.

#![cfg(test)]

use model::compare::ComparisonPart;
use model::contract::Target;
use model::diff::DiffSide;
use model::state::ChangeKind;
use protocol::feature::TargetJson;
use protocol::messages::{DiscoverParams, DiscoverResult, DiscoveredRepositoryJson};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;

/// Writes a value and reads it back, which must give the same value.
fn round_trip<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: &T) {
    let text = serde_json::to_string(value).unwrap();
    let read: T = serde_json::from_str(&text).unwrap();
    assert_eq!(&read, value, "{text}");
}

#[test]
fn the_repositories_of_a_workspace_are_written_with_their_superproject_and_common_folder() {
    round_trip(&DiscoverParams {
        folders: vec![String::from("/work")],
    });
    let result = DiscoverResult {
        repositories: vec![
            DiscoveredRepositoryJson {
                folder: String::from("/work/app"),
                superproject: None,
                common_folder: Some(String::from("/work/app/.git")),
            },
            DiscoveredRepositoryJson {
                folder: String::from("/work/app/lib"),
                superproject: Some(String::from("/work/app")),
                common_folder: None,
            },
        ],
    };
    round_trip(&result);
    assert_eq!(
        serde_json::to_value(&result).unwrap()["repositories"][0],
        json!({ "folder": "/work/app", "superproject": null, "commonFolder": "/work/app/.git" })
    );
}

#[test]
fn a_file_of_a_comparison_is_the_part_of_the_comparison_it_was_invoked_on() {
    let target: TargetJson = serde_json::from_value(json!({
        "kind": "comparison",
        "first": { "name": "Working tree", "side": { "kind": "workingTree" }, "isFollowed": true, "isGone": false },
        "second": { "name": "main", "side": { "kind": "commit", "commit": "a".repeat(40) }, "isFollowed": true, "isGone": false },
        "mode": "everything",
        "path": null,
        "mergeBase": null,
        "file": { "path": "src/app.rs", "originalPath": null, "kind": "deleted" }
    }))
    .unwrap();
    let Target::Comparison(comparison) = Target::try_from(target).unwrap() else {
        panic!("not a comparison");
    };
    let ComparisonPart::File(file) = &comparison.part else {
        panic!("not a file of the comparison");
    };
    assert_eq!((file.path.as_str(), file.kind), ("src/app.rs", ChangeKind::Deleted));
}

#[test]
fn a_change_of_a_diff_is_read_with_its_sides_lines_and_text() {
    let target: TargetJson = serde_json::from_value(json!({
        "kind": "diffChange",
        "path": "src/app.rs",
        "left": { "kind": "head" },
        "right": { "kind": "workingTree" },
        "leftLines": { "start": 2, "end": 3 },
        "rightLines": { "start": 2, "end": 4 },
        "leftText": ["old"],
        "rightText": ["new", "more"]
    }))
    .unwrap();
    round_trip(&target);
    let Target::DiffChange(change) = Target::try_from(target).unwrap() else {
        panic!("not a change of a diff");
    };
    assert_eq!((change.left.clone(), change.right.clone()), (DiffSide::Head, DiffSide::WorkingTree));
    assert_eq!((change.left_lines.clone(), change.right_lines.clone()), (2..3, 2..4));
    assert_eq!(change.right_text, ["new", "more"]);
    let commit: TargetJson = serde_json::from_value(json!({
        "kind": "diffChange",
        "path": "a.txt",
        "left": { "kind": "commit", "commit": "nope" },
        "right": { "kind": "index" },
        "leftLines": { "start": 0, "end": 0 },
        "rightLines": { "start": 0, "end": 1 },
        "leftText": [],
        "rightText": ["a"]
    }))
    .unwrap();
    assert!(Target::try_from(commit).is_err());
}
