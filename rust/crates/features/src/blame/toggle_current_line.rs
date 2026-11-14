//! `blame.toggle-current-line`: turns the blame of the line under the cursor on or off, in the user's settings.

use model::contract::{Answers, Context, Outcome};

use super::switch;

/// Sets `blame.enabled` to the opposite of what it is in the user's settings file, keeping the rest of the file.
///
/// Feature: blame.toggle-current-line
#[must_use]
pub fn toggle_current_line(context: &Context, answers: &Answers) -> Outcome {
    switch(context, answers, ("blame.enabled", context.settings.blame.is_enabled))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answer, Answers, Outcome, Query, Step};

    use super::toggle_current_line;
    use crate::blame::SETTINGS;
    use model::blame::ShownBlame;

    use crate::blame::testing::on_line;
    use crate::testing::plan;

    /// What the plan writes in the settings file, which held a text before.
    fn written(is_enabled: bool, before: Option<&str>) -> String {
        let mut context = on_line(1, ShownBlame::Hidden);
        context.settings.blame.is_enabled = is_enabled;
        let Outcome::Read(SETTINGS, Query::FileTexts { paths }) = toggle_current_line(&context, &Answers::new()) else {
            panic!("reads the settings file first");
        };
        assert_eq!(paths, [std::path::PathBuf::from("settings.json")]);
        let read = Answer::Files(vec![before.map(|text| text.as_bytes().to_vec())]);
        let plan = plan(toggle_current_line(&context, &Answers::new().with(SETTINGS, read)));
        let [Step::Write { path, contents }] = plan.steps.as_slice() else {
            panic!("writes the settings file: {plan:?}");
        };
        assert_eq!(path, &std::path::PathBuf::from("settings.json"));
        String::from_utf8(contents.clone()).unwrap()
    }

    #[test]
    fn blame_toggle_current_line__sets_blame_enabled_to_false_in_the_user_settings() {
        let before = "{\n  // The theme.\n  \"theme\": \"dark\"\n}\n";
        let after = written(true, Some(before));
        assert!(json::parse_with_comments(&after).is_ok(), "{after}");
        assert!(after.contains("// The theme."), "{after}");
        assert!(after.contains("\"blame.enabled\": false"), "{after}");
    }

    #[test]
    fn blame_toggle_current_line__run_again_sets_it_to_true() {
        let after = written(false, Some("{ \"blame.enabled\": false }"));
        assert!(after.contains("\"blame.enabled\": true"), "{after}");
        assert!(!after.contains("false"), "{after}");
        assert!(written(false, None).contains("\"blame.enabled\": true"));
    }
}
