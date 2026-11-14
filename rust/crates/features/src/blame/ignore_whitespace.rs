//! `blame.ignore-whitespace`: leaves a line to who wrote it when all that changed since is its whitespace or its place,
//! by switching `blame.ignoreWhitespaceAndMoves` in the user's settings.

use model::contract::{Answers, Context, Outcome};

use super::switch;

/// Sets `blame.ignoreWhitespaceAndMoves` to the opposite of what it is in the user's settings file; the blame of every
/// file that shows one, and of the line under the cursor, is then read again with or without `-w -M`.
///
/// Feature: blame.ignore-whitespace
#[must_use]
pub fn ignore_whitespace(context: &Context, answers: &Answers) -> Outcome {
    let is_on = context.settings.blame.ignores_whitespace;
    switch(context, answers, ("blame.ignoreWhitespaceAndMoves", is_on))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answer, Answers, Step};

    use super::ignore_whitespace;
    use crate::blame::SETTINGS;
    use model::blame::ShownBlame;

    use crate::blame::testing::on_line;
    use crate::testing::plan;

    #[test]
    fn blame_ignore_whitespace__writes_the_opposite_of_the_setting() {
        let mut context = on_line(1, ShownBlame::Hidden);
        let read = Answers::new().with(SETTINGS, Answer::Files(vec![None]));
        for (is_on, said) in [(false, "true"), (true, "false")] {
            context.settings.blame.ignores_whitespace = is_on;
            let plan = plan(ignore_whitespace(&context, &read));
            let [Step::Write { contents, .. }] = plan.steps.as_slice() else {
                panic!("writes the settings file");
            };
            let text = String::from_utf8(contents.clone()).unwrap();
            assert!(
                text.contains(&format!("\"blame.ignoreWhitespaceAndMoves\": {said}")),
                "{text}"
            );
        }
    }
}
