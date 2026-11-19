//! The read of the tags of a remote that version 1.13 adds for milestone M35 (`tag.read-remote-tags`), against a bare
//! repository beside the one read, never the network.

use serde_json::json;
use testkit::Remote;

use crate::harness::repository;
use crate::reads::{opened, read, result};

#[test]
fn the_tags_of_origin_are_compared_with_those_of_the_repository_and_a_remote_that_cannot_be_reached_says_so() {
    let mut repository = repository();
    let _origin = Remote::new(&repository);
    let _ = repository.git(&["push", "--quiet", "origin", "refs/tags/v1.0"]);
    let _ = repository.commit("Add b", &[("b.txt", "b\n")]);
    repository.tag("v2.0");
    let mut engine = opened(&repository);
    let compared = result(
        &mut engine,
        &repository,
        "read/tag.read-remote-tags",
        json!({ "remote": "origin" }),
    );
    assert_eq!(compared["remote"], json!("origin"));
    assert_eq!(
        compared["tags"],
        json!([
            { "name": "v1.0", "onRemote": "pushed" },
            { "name": "v2.0", "onRemote": "notPushed" }
        ]),
        "{compared}"
    );
    assert_eq!(compared["onlyThere"], json!([]));
    let _ = repository.git(&["remote", "add", "gone", "../nowhere.git"]);
    let failed = read(
        &mut engine,
        &repository,
        "read/tag.read-remote-tags",
        json!({ "remote": "gone" }),
    );
    assert!(failed.get("error").is_some(), "{failed}");
}
