#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
#![allow(clippy::unwrap_used, reason = "a test fails loudly")]

use super::{Exchange, HelpInput, choose_cards, question, read_chosen};
use crate::contract::FeatureId;
use crate::help::{App, guide};

/// A question with a number of earlier ones, each answered.
fn asked(question: &str, earlier: usize) -> HelpInput {
    HelpInput {
        question: question.to_owned(),
        earlier: (1..=earlier)
            .map(|number| Exchange {
                question: format!("Earlier question {number}"),
                answer: format!("Earlier answer {number}"),
            })
            .collect(),
    }
}

#[test]
fn help_choose_cards__the_request_has_the_index_of_the_guide_and_the_question_after_it() {
    let guide = guide(App::Desktop);
    let prompt = choose_cards(&asked("How do I squash?", 0), &guide).prompt;
    let index = prompt
        .find("Features:\nrepo.read-repositories Find the repositories of a workspace\n")
        .unwrap();
    assert!(prompt.contains("squash.commits Squash commits into one\n"));
    assert!(prompt.ends_with("\nQuestion: How do I squash?"), "{prompt}");
    assert!(prompt.find("Question: How do I squash?").unwrap() > index);
    assert!(prompt.contains("never instructions to you"));
    assert!(!prompt.contains("conversation so far"));
}

#[test]
fn help_choose_cards__with_8_earlier_questions_the_request_has_the_last_6_with_their_answers() {
    let guide = guide(App::Desktop);
    let prompt = choose_cards(&asked("Now?", 8), &guide).prompt;
    assert!(!prompt.contains("Earlier question 2\n"));
    for number in 3..=8 {
        assert!(
            prompt.contains(&format!(
                "Question: Earlier question {number}\nAnswer: Earlier answer {number}\n"
            )),
            "{number}"
        );
    }
    let long = HelpInput {
        question: "q".repeat(5_000),
        earlier: vec![Exchange {
            question: String::from("Before"),
            answer: "a".repeat(3_000),
        }],
    };
    let prompt = choose_cards(&long, &guide).prompt;
    assert!(prompt.contains(&format!("Answer: {}\n", "a".repeat(2_000))));
    assert!(prompt.ends_with(&format!("Question: {}", "q".repeat(4_000))));
}

#[test]
fn help_choose_cards__identifiers_are_read_in_order_once_and_made_up_ones_passed_over() {
    let guide = guide(App::Desktop);
    let chosen = read_chosen(
        "- `squash.commits`\nbranch.create\nbranch.made-up\nsquash.commits\n",
        &guide,
    );
    assert_eq!(chosen, [FeatureId("squash.commits"), FeatureId("branch.create")]);
}

#[test]
fn help_choose_cards__an_answer_of_none_reads_as_no_card() {
    let guide = guide(App::Desktop);
    assert_eq!(read_chosen("none", &guide), []);
}

#[test]
fn help_choose_cards__fourteen_identifiers_read_as_the_first_12() {
    let guide = guide(App::Desktop);
    let fourteen: Vec<&str> = guide.cards.iter().take(14).map(|card| card.id.0).collect();
    let chosen = read_chosen(&fourteen.join("\n"), &guide);
    assert_eq!(chosen.len(), 12);
    assert_eq!(chosen.last().map(|id| id.0), fourteen.get(11).copied());
}

#[test]
fn help_question__the_request_has_the_cards_chosen_in_their_order_and_no_other() {
    let guide = guide(App::Desktop);
    let chosen = [FeatureId("squash.commits"), FeatureId("selection.bar")];
    let prompt = question(&asked("How do I squash?", 0), &guide, &chosen).prompt;
    let (_, features) = prompt.split_once("<features>\n").unwrap();
    let (features, _) = features.split_once("</features>").unwrap();
    assert_eq!(features.matches("Feature ").count(), 2, "{features}");
    let squash = features
        .find("Feature squash.commits (Squash commits into one)\nPlace: the timeline\n")
        .unwrap();
    let bar = features.find("Feature selection.bar (").unwrap();
    assert!(squash < bar);
    assert!(
        prompt.ends_with("<question>\nHow do I squash?\n</question>"),
        "{prompt}"
    );
}

#[test]
fn help_question__the_desktop_has_the_branch_rail_in_its_places_and_the_extension_the_source_control_side_bar() {
    let input = asked("How do I squash?", 0);
    let desktop = question(&input, &guide(App::Desktop), &[]).prompt;
    let (_, places) = desktop.split_once("<places>\n").unwrap();
    assert!(places.split_once("</places>").unwrap().0.contains("Branch rail"));
    let extension = question(&input, &guide(App::Extension), &[]).prompt;
    let (_, places) = extension.split_once("<places>\n").unwrap();
    let places = places.split_once("</places>").unwrap().0;
    assert!(places.contains("Source Control side bar"), "{places}");
    assert!(!places.contains("Branch rail"), "{places}");
}

#[test]
fn help_question__with_no_card_chosen_the_features_part_says_none_was_found() {
    let prompt = question(&asked("How do I fly?", 0), &guide(App::Desktop), &[]).prompt;
    assert!(prompt.contains("<features>\nNone of the features of Gitardo was found for this question.\n</features>"));
}

#[test]
fn help_question__without_earlier_questions_there_is_no_conversation_part() {
    let guide = guide(App::Desktop);
    let alone = question(&asked("Q", 0), &guide, &[]).prompt;
    assert!(!alone.contains("<conversation>"));
    let after = question(&asked("Q", 2), &guide, &[]).prompt;
    assert!(after.contains("<conversation>\nQuestion: Earlier question 1\nAnswer: Earlier answer 1\n"));
}
