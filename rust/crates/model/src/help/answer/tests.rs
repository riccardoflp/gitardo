#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::{read_answer, shown_feature};
use crate::contract::FeatureId;
use crate::help::{App, guide};

#[test]
fn help_read_answer__a_feature_of_the_guide_reads_as_the_title_of_its_card_as_a_link_and_is_named() {
    let answer = read_answer("Select the commits, then [[squash.commits]].", &guide(App::Desktop));
    assert_eq!(
        answer.markdown,
        "Select the commits, then [Squash commits into one](show-me:squash.commits)."
    );
    assert_eq!(answer.named, [FeatureId("squash.commits")]);
    assert_eq!(shown_feature("show-me:squash.commits"), Some("squash.commits"));
}

#[test]
fn help_read_answer__a_feature_the_guide_does_not_have_is_taken_out_and_names_nothing() {
    let answer = read_answer("Use [[branch.made-up]] to do it.", &guide(App::Desktop));
    assert_eq!(answer.markdown, "Use to do it.");
    assert_eq!(answer.named, []);
}

#[test]
fn help_read_answer__a_feature_named_twice_is_named_once_where_it_was_first_named() {
    let answer = read_answer(
        "```markdown\n1. [[branch.create]], then [[squash.commits]].\n2. [[branch.create]] again.\n```",
        &guide(App::Desktop),
    );
    assert_eq!(answer.named, [FeatureId("branch.create"), FeatureId("squash.commits")]);
    assert!(
        answer.markdown.starts_with("1. [Create a branch]"),
        "{}",
        answer.markdown
    );
}

#[test]
fn help_read_answer__a_piece_ending_in_half_a_feature_shows_the_text_before_it() {
    let guide = guide(App::Desktop);
    let answer = read_answer("Select the commits, then [[squash.co", &guide);
    assert_eq!(answer.markdown, "Select the commits, then");
    assert_eq!(answer.named, []);
    let coming = read_answer("```\nSelect the commits", &guide);
    assert_eq!(coming.markdown, "Select the commits");
}
