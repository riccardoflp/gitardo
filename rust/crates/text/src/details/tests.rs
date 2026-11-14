#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::{RawMessage, ShownMessage, message_title, unwrap_message};

/// The message shown for a raw message.
fn shown(text: &str) -> ShownMessage {
    message_title(&RawMessage { text })
}

/// A paragraph of prose cut at 72 columns.
const WRAPPED: &str = "Read the configuration once at startup and keep it in the store of the\n\
window, so that every place reads the same values and none of them goes\n\
to the disk again while a frame is drawn.";

#[test]
fn details_message_title__a_subject_of_100_characters_or_fewer_is_the_title_unchanged() {
    let message = shown("Fix the parser");
    assert_eq!(message.title, "Fix the parser");
    assert_eq!(message.text, "");
    let hundred = "a".repeat(50) + " " + &"b".repeat(49);
    let message = shown(&format!("{hundred}\n\nThe body."));
    assert_eq!(message.title, hundred);
    assert_eq!(message.text, "The body.");
}

#[test]
fn details_message_title__a_one_paragraph_message_gets_its_first_sentence_as_the_title() {
    let rest = "Then every place reads the same values and none of them goes to the disk again while a frame is drawn.";
    let message = shown(&format!(
        "Read the configuration once at startup. {rest}\n\nSigned-off-by: Someone <a@b.c>"
    ));
    assert_eq!(message.title, "Read the configuration once at startup.");
    assert_eq!(message.text, format!("{rest}\n\nSigned-off-by: Someone <a@b.c>"));
    let wrapped = shown(WRAPPED);
    assert!(wrapped.title.ends_with('\u{2026}'), "{}", wrapped.title);
    assert!(!wrapped.subject.contains('\n'));
}

#[test]
fn details_message_title__a_first_sentence_shorter_than_20_characters_is_not_taken() {
    let tail = "X ".repeat(60);
    let message = shown(&format!("Fix it. Read the files once. {tail}"));
    assert_eq!(message.title, "Fix it. Read the files once.");
    assert!(message.text.starts_with("X X"));
}

#[test]
fn details_message_title__a_sentence_ending_after_120_characters_is_cut_at_a_space_instead() {
    let long = "word ".repeat(26);
    let message = shown(&format!("{}. And then more of it.", long.trim_end()));
    assert!(message.title.ends_with('\u{2026}'));
    assert!(message.title.chars().count() <= 81);
    assert!(message.text.starts_with('\u{2026}'));
    let no_space = "a".repeat(150);
    assert_eq!(shown(&no_space).title, no_space);
}

#[test]
fn details_message_title__a_list_under_the_first_line_goes_to_the_body() {
    let message = shown("Change the settings\n- read them once\n- write them on a thread");
    assert_eq!(message.title, "Change the settings");
    assert_eq!(message.body, "- read them once\n- write them on a thread");
    assert_eq!(message.text, message.body);
}

#[test]
fn details_message_title__a_dot_before_a_digit_or_a_lower_case_letter_does_not_end_a_sentence() {
    let start = "Update the reader to v1.2 so that e.g. this case works for the people who use it";
    let message = shown(&format!("{start} every day. Then the rest follows here."));
    assert_eq!(message.title, format!("{start} every day."));
    assert_eq!(message.text, "Then the rest follows here.");
}

#[test]
fn details_message_unwrap__a_paragraph_wrapped_at_72_columns_becomes_one_line() {
    let unwrapped = unwrap_message(WRAPPED);
    assert_eq!(unwrapped.lines().count(), 1);
    let first = "a".repeat(5) + &" word".repeat(13);
    let second = "tail of the paragraph that goes on";
    assert_eq!(
        unwrap_message(&format!("{first}\n{second}")),
        format!("{first} {second}")
    );
}

#[test]
fn details_message_unwrap__a_message_whose_longest_line_is_shorter_than_50_is_unchanged() {
    let message = "Keep the lines\nas they were written\nby hand";
    assert_eq!(unwrap_message(message), message);
}

#[test]
fn details_message_unwrap__a_line_that_ends_early_is_not_joined_to_the_next() {
    let message = format!(
        "{}\nA line of forty characters, ending here.\nAnd another one after it.",
        "word ".repeat(14).trim_end()
    );
    let unwrapped = unwrap_message(&message);
    assert!(unwrapped.contains("ending here.\nAnd another"));
}

#[test]
fn details_message_unwrap__lists_indented_lines_quotations_headings_fences_addresses_and_trailers_stay() {
    let long = "word ".repeat(14);
    let long = long.trim_end();
    let message = format!(
        "{long}\n- first item\n- second item\n\n{long}\n    indented on purpose\n\n{long}\n> quoted text\n\n\
         {long}\n# Heading\n\n```\n{long}\nfenced text\n```\n\n{long}\nhttps://example.com/a/long/address\n\n\
         Co-Authored-By: Someone <someone@example.com>\nSigned-off-by: Someone <someone@example.com>"
    );
    assert_eq!(unwrap_message(&message), message);
}

#[test]
fn details_message_unwrap__the_continuation_of_a_list_item_is_joined_to_it() {
    let item = format!("- {}", "word ".repeat(13).trim_end());
    let message = format!("{item}\n  continued here\n- the next item");
    assert_eq!(
        unwrap_message(&message),
        format!("{item} continued here\n- the next item")
    );
}

#[test]
fn details_message_unwrap__no_character_but_the_line_breaks_joined_changes() {
    let message = format!(
        "{WRAPPED}\n\n- {}\n  goes on\n\nSigned-off-by: A <a@b.c>",
        "item ".repeat(14).trim_end()
    );
    let unwrapped = unwrap_message(&message);
    let squeezed = |text: &str| {
        text.chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>()
    };
    assert_eq!(squeezed(&unwrapped), squeezed(&message));
    assert!(unwrapped.lines().count() < message.lines().count());
}
