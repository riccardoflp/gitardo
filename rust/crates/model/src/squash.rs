//! The message of the commit that several commits squashed together become.

use crate::rebase::body_of;

/// The subject of a message: its first paragraph, on one line.
fn subject_of(message: &str) -> String {
    message
        .trim()
        .lines()
        .take_while(|line| !line.trim().is_empty())
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ")
}

/// The message of commits squashed into one: the subject given, then each message, joined by blank lines. A message
/// whose subject is the one given adds only its body; an empty part adds nothing.
///
/// Feature: squash.join-messages
#[must_use]
pub fn join_messages(subject: &str, messages: &[&str]) -> String {
    let subject = subject.trim();
    let mut parts = vec![subject];
    for message in messages {
        let part = if subject_of(message) == subject {
            body_of(message)
        } else {
            message.trim()
        };
        parts.push(part);
    }
    parts.retain(|part| !part.is_empty());
    parts.join("\n\n")
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::join_messages;

    #[test]
    fn squash_join_messages__a_message_with_the_subject_given_adds_its_body() {
        assert_eq!(
            join_messages(
                "Add the login page",
                &["Add the login page\n\nWith a form.\n", "Fix a typo\n"]
            ),
            "Add the login page\n\nWith a form.\n\nFix a typo"
        );
    }

    #[test]
    fn squash_join_messages__messages_with_no_body_follow_a_new_subject() {
        assert_eq!(
            join_messages(
                " Rework the header ",
                &["Move the logo", "Shrink the menu\n", "Fix the colors"]
            ),
            "Rework the header\n\nMove the logo\n\nShrink the menu\n\nFix the colors"
        );
    }

    #[test]
    fn a_subject_on_two_lines_is_one_subject() {
        assert_eq!(
            join_messages("Add the login page", &["Add the login\npage\n\nWith a form."]),
            "Add the login page\n\nWith a form."
        );
    }
}
