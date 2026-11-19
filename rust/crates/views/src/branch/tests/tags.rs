//! The entries of the menu of a tag: checking it out, and deleting it from a remote (`tag.delete-remote`).

use super::{of_tag, repository};

#[test]
fn tag_checkout__a_tag_on_the_commit_checked_out_has_no_such_entry() {
    let repository = repository();
    assert_eq!(
        of_tag(&repository, "v1"),
        [
            "Create branch here\u{2026}",
            "Push tag",
            "Delete",
            "Delete from origin",
            "Delete here and from origin"
        ]
    );
}

#[test]
fn tag_delete_remote__with_several_remotes_the_entries_ask_which_and_with_none_there_is_neither() {
    let mut repository = repository();
    repository.remotes.push(String::from("upstream"));
    let entries = of_tag(&repository, "v1");
    assert!(
        entries.contains(&String::from("Delete from a remote\u{2026}")),
        "{entries:?}"
    );
    assert!(
        entries.contains(&String::from("Delete here and from a remote\u{2026}")),
        "{entries:?}"
    );
    repository.remotes.clear();
    let entries = of_tag(&repository, "v1");
    assert!(
        !entries.iter().any(|entry| entry.starts_with("Delete from")),
        "{entries:?}"
    );
}
