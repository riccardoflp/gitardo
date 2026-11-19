//! The operations on the notes in the menu of a remote in the branch rail (`note.fetch`, `note.push`).

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::contract::{FeatureId, Target};

use super::menu::list_operations;
use super::tests::{Rail, id};
use crate::intent::RailList;

#[test]
fn note_push__the_menu_of_a_remote_offers_fetch_notes_and_push_notes_only_with_notes() {
    let mut rail = Rail::new(&["main"], &["origin/main"], &[]);
    let origin = RailList::Remote(String::from("origin"));
    let labels = |rail: &Rail| -> Vec<String> {
        list_operations(&rail.state(), &origin)
            .into_iter()
            .map(|entry| entry.label)
            .collect()
    };
    assert_eq!(labels(&rail), ["Fetch notes"]);
    rail.repository.notes.tip = Some(id("notes"));
    assert_eq!(labels(&rail), ["Fetch notes", "Push notes"]);
    let operations = list_operations(&rail.state(), &origin);
    let run = operations.last().and_then(|entry| entry.run.clone());
    assert_eq!(
        run,
        Some((FeatureId("note.push"), Target::Remote(String::from("origin"))))
    );
}
