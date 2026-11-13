//! `blame.every-file`: shows the blame in every file opened, by switching `fileBlame.everyFile` in the user's settings.

use model::contract::{Answers, Context, Outcome};

use super::switch;

/// Sets `fileBlame.everyFile` to the opposite of what it is in the user's settings file: on, every File page of a file
/// on disk shows its blame unless it was hidden in it; off, the pages that showed it only because of the setting lose
/// it.
///
/// Feature: blame.every-file
#[must_use]
pub fn every_file(context: &Context, answers: &Answers) -> Outcome {
    switch(
        context,
        answers,
        ("fileBlame.everyFile", context.settings.blame.is_on_every_file),
    )
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answer, Answers, Outcome, Step};

    use super::every_file;
    use crate::blame::SETTINGS;
    use model::blame::ShownBlame;

    use crate::blame::testing::on_line;
    use crate::testing::plan;

    #[test]
    fn blame_every_file__writes_the_opposite_of_the_setting_and_refuses_with_no_settings_file() {
        let mut context = on_line(1, ShownBlame::Hidden);
        let read = Answers::new().with(SETTINGS, Answer::Files(vec![Some(b"{}".to_vec())]));
        let plan = plan(every_file(&context, &read));
        let [Step::Write { contents, .. }] = plan.steps.as_slice() else {
            panic!("writes the settings file");
        };
        let text = String::from_utf8(contents.clone()).unwrap();
        assert!(text.contains("\"fileBlame.everyFile\": true"), "{text}");
        context.settings.blame.user_file = None;
        assert!(matches!(every_file(&context, &read), Outcome::Refuse(_)));
    }
}
