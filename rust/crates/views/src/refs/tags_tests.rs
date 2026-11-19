//! The menu of the Tags list of the branch rail (`tag.remote-marks`, `tag.push-all`): **Check tags on `<remote>`** and
//! **Push tags…**, as the remote and the marks of the tags say.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::refs::SortOrder;
use ui::MenuEntry;

use super::ReferencesState;
use super::tests::Rail;
use crate::intent::RailList;

/// The words of the entries of the menu of the Tags list.
fn tags_menu_words(rail: &Rail, marks: Option<&model::tag::TagMarks>) -> Vec<String> {
    let state = ReferencesState {
        tag_marks: marks,
        ..rail.state()
    };
    let tags = super::menu::tags_menu(&state);
    super::menu::list_menu(&RailList::Tags, (SortOrder::Name, &tags), &[])
        .iter()
        .filter_map(|entry| match entry {
            MenuEntry::Item(item) => Some(item.label.to_owned()),
            _ => None,
        })
        .collect()
}

#[test]
fn tag_remote_marks__the_menu_of_the_tags_checks_origin_and_offers_push_tags_while_one_is_not_on_it() {
    let mut rail = Rail::new(&["main"], &["origin/main"], &["v1.0"]);
    rail.repository.remotes.clear();
    assert_eq!(
        tags_menu_words(&rail, None),
        ["Sort by name", "Sort by date"],
        "no remote"
    );
    rail.repository.remotes = vec![String::from("origin")];
    let every = [
        "Sort by name",
        "Sort by date",
        "Check tags on origin",
        "Push tags\u{2026}",
    ];
    assert_eq!(tags_menu_words(&rail, None), every, "before a check");
    let tag = rail.references.tags.first().cloned().unwrap_or_else(|| panic!("a tag"));
    let mut marks = model::tag::TagMarks {
        remote: String::from("origin"),
        tags: Vec::new(),
    };
    assert_eq!(tags_menu_words(&rail, Some(&marks)), every, "v1.0 is not on origin");
    marks.pushed(&tag.name, &tag.commit);
    assert_eq!(
        tags_menu_words(&rail, Some(&marks)),
        every[..3],
        "every tag is on origin"
    );
}
