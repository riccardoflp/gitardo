//! The marks of the tags a remote lacks, from marks made by hand.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::state::Tag;
use model::tag::{RemoteTags, TagMarks, TagObject, compare_remote};
use model::{CommitId, Timestamp};

use super::{label_icons, not_pushed_words, remote_marks_view};
use crate::icons;

fn id(digit: char) -> CommitId {
    CommitId::parse(&digit.to_string().repeat(40)).unwrap_or_else(|| panic!("a hash"))
}

fn tag(name: &str, digit: char) -> Tag {
    Tag {
        name: name.to_owned(),
        commit: id(digit),
        date: Timestamp::default(),
    }
}

/// The tags `v1`, pushed, and `v2`, made here; and the marks of a check of `origin`, which has `v1` alone.
fn checked() -> ([Tag; 2], TagMarks) {
    let tags = [tag("v1", '1'), tag("v2", '2')];
    let object = |name: &str, digit: char| TagObject {
        name: name.to_owned(),
        object: digit.to_string().repeat(40),
    };
    let there = RemoteTags {
        remote: String::from("origin"),
        there: vec![object("v1", '1')],
        here: Vec::new(),
    };
    let compared = compare_remote(&[object("v1", '1'), object("v2", '2')], &there);
    let marks = TagMarks::checked(&compared, &tags);
    (tags, marks)
}

#[test]
fn tag_remote_marks__before_a_check_no_tag_is_marked() {
    let (tags, _) = checked();
    assert_eq!(remote_marks_view(None, (&tags[1].name, &tags[1].commit)), None);
    assert_eq!(not_pushed_words(None, &tags), None);
    assert_eq!(label_icons(None), [icons::TAG]);
}

#[test]
fn tag_remote_marks__the_check_marks_the_tag_not_on_origin_alone_and_the_title_counts_it() {
    let (tags, marks) = checked();
    assert_eq!(remote_marks_view(Some(&marks), (&tags[0].name, &tags[0].commit)), None);
    let mark = remote_marks_view(Some(&marks), (&tags[1].name, &tags[1].commit));
    assert_eq!(mark, Some((icons::NOT_ON_REMOTE, String::from("Not on origin"))));
    assert_eq!(
        label_icons(mark.map(|(icon, _)| icon)),
        [icons::TAG, icons::NOT_ON_REMOTE]
    );
    assert_eq!(
        not_pushed_words(Some(&marks), &tags).as_deref(),
        Some("\u{b7} 1 not on origin")
    );
}

#[test]
fn tag_remote_marks__a_tag_moved_after_it_was_pushed_says_origin_has_another_tag() {
    let (_, marks) = checked();
    let moved = tag("v1", '9');
    assert_eq!(
        remote_marks_view(Some(&marks), (&moved.name, &moved.commit)),
        Some((
            icons::DIFFERENT_ON_REMOTE,
            String::from("origin has another tag of this name")
        ))
    );
}
