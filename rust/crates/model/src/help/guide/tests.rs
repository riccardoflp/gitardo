#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
#![allow(clippy::unwrap_used, reason = "a test fails loudly")]

use super::{App, Show, guide, read_guide};

/// A guide of two cards in the desktop and one in the extension.
const SMALL: &str = "\
The guide of Ask Gitardo.
@places desktop
Branch rail: the branches.
@places extension
Source Control side bar: the views.
@card desktop stash.push
title: Stash changes
place: the draft
show: quick search
Puts the changes aside.
Entry points: **Stash Changes** in Quick search.
@card desktop file-search.page
title: Search in files
place: the Search page
show: page
Finds lines.
@card extension stash.push
title: Stash changes
place: the draft
show: quick search
Puts the changes aside.
In VS Code: **Gitardo: Stash Changes** in the Command Palette.
";

#[test]
fn help_guide__the_guide_of_an_app_has_its_places_and_only_its_cards() {
    let desktop = read_guide(SMALL, App::Desktop);
    assert_eq!(desktop.places, "Branch rail: the branches.");
    assert_eq!(
        desktop.index(),
        "stash.push Stash changes\nfile-search.page Search in files\n"
    );
    let stash = desktop.card("stash.push").unwrap();
    assert_eq!(stash.show, Show::QuickSearch);
    assert_eq!(
        stash.text,
        "Puts the changes aside.\nEntry points: **Stash Changes** in Quick search."
    );
    assert_eq!(desktop.card("file-search.page").unwrap().show, Show::Page);
    let extension = read_guide(SMALL, App::Extension);
    assert_eq!(extension.places, "Source Control side bar: the views.");
    assert_eq!(extension.cards.len(), 1);
    assert!(extension.cards[0].text.ends_with("in the Command Palette."));
}

#[test]
fn help_guide__the_guide_held_in_the_program_has_squash_commits_in_the_timeline() {
    let desktop = guide(App::Desktop);
    let squash = desktop.card("squash.commits").unwrap();
    assert_eq!(squash.place, "the timeline");
    assert!(
        squash.text.contains("**Squash into one** in the selection bar"),
        "{}",
        squash.text
    );
    assert!(desktop.places.contains("Branch rail"), "{}", desktop.places);
    let extension = guide(App::Extension);
    assert!(
        extension.places.contains("Source Control side bar"),
        "{}",
        extension.places
    );
    assert!(extension.cards.len() > 100);
}
