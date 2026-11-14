//! The dialogs of an operation run from the window: the lines of a plan as they are shown before it runs.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::PathBuf;

use model::contract::{Danger, Plan, Step, Text, Undo};

use super::plan_lines;
use crate::text::feature as words;

#[test]
fn preview_confirm__the_patch_given_on_standard_input_is_shown_under_its_command() {
    let plan = Plan {
        title: Text::from("Stage a change of src/a.rs"),
        steps: vec![Step::Git {
            arguments: ["apply", "--cached", "-"].into_iter().map(Into::into).collect(),
            directory: PathBuf::from("/work/app"),
            input: Some(b"--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1 +1 @@\n-old\n+new\n".to_vec()),
        }],
        danger: Danger::None,
        undo: Undo::ByReflog,
        after: Vec::new(),
        on_failure: Vec::new(),
    };
    assert_eq!(
        plan_lines(&plan),
        [
            "git apply --cached -",
            "  --- a/src/a.rs",
            "  +++ b/src/a.rs",
            "  @@ -1 +1 @@",
            "  -old",
            "  +new"
        ]
    );
}

#[test]
fn a_reason_is_said_as_a_sentence_before_the_commands() {
    assert_eq!(
        words::warning("It moves main"),
        "It moves main. These commands will run:"
    );
    assert_eq!(
        words::warning("It moves main."),
        "It moves main. These commands will run:"
    );
}
