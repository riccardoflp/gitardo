//! `merge-view.mark-resolved`: **Mark as resolved** at the bottom of the result of a Merge page.

use model::contract::{
    AnswerKey, Answers, Context, Danger, FollowUp, Outcome, Plan, Question, QuestionKind, Refusal, Step, Target, Text,
    Undo,
};
use model::merge_view::MergeResolution;

/// The key of the question asked when git's markers are left in the result.
pub const MARK_ANYWAY: AnswerKey = AnswerKey("mark anyway");

/// Said when the operation is given no file of a Merge page.
const CHOOSE_A_FILE: &str = "Choose the file of a Merge page.";
/// Why Undo does not take it back.
const NOT_UNDONE: &str = "Staging leaves nothing in the reflog.";

/// Saves the file when the result has changes that are not saved, then stages it, which tells git that its conflicts
/// are resolved; the page closes. Asks first when git's markers are left in it, since staged they would be committed.
///
/// Feature: merge-view.mark-resolved
#[must_use]
pub fn mark_resolved(context: &Context, answers: &Answers) -> Outcome {
    let Target::MergeResult(resolution) = &context.target else {
        return Outcome::Refuse(Refusal::saying(CHOOSE_A_FILE));
    };
    if resolution.marked > 0 && !answers.is_confirmed(MARK_ANYWAY) {
        return Outcome::Ask(markers_question(resolution));
    }
    let folder = context.repository.folder.clone();
    let mut steps = Vec::new();
    if let Some(contents) = &resolution.unsaved {
        let path = resolution
            .path
            .split('/')
            .fold(folder.clone(), |path, name| path.join(name));
        steps.push(Step::Write {
            path,
            contents: contents.clone(),
        });
    }
    steps.push(crate::conflicts::staging(folder, &resolution.path));
    Outcome::Run(Plan {
        title: Text::from(format!("Mark {} as resolved", resolution.path)),
        steps,
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(NOT_UNDONE)),
        after: vec![FollowUp::CloseMergePage(resolution.path.clone())],
        on_failure: Vec::new(),
    })
}

/// **app.rs still has 2 conflicts marked by git. Mark it as resolved all the same, with the markers in it?**
fn markers_question(resolution: &MergeResolution) -> Question {
    let name = resolution.path.rsplit('/').next().unwrap_or(&resolution.path);
    let count = if resolution.marked == 1 {
        String::from("1 conflict")
    } else {
        format!("{} conflicts", resolution.marked)
    };
    Question {
        key: MARK_ANYWAY,
        kind: QuestionKind::Confirm {
            accept: Text::from("Mark as resolved"),
        },
        title: Text::from(format!(
            "{name} still has {count} marked by git. Mark it as resolved all the same, with the markers in it?"
        )),
        detail: None,
        default: None,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{MARK_ANYWAY, mark_resolved};
    use crate::testing::{context, lines, plan, question, state};
    use model::contract::{Answer, Answers, FollowUp, Outcome, QuestionKind, Target};
    use model::merge_view::MergeResolution;

    fn target(unsaved: Option<&str>, marked: usize) -> Target {
        Target::MergeResult(Box::new(MergeResolution {
            path: String::from("src/app.rs"),
            unsaved: unsaved.map(|text| text.as_bytes().to_vec()),
            marked,
        }))
    }

    #[test]
    fn merge_view_mark_resolved__an_unsaved_result_is_written_then_staged_and_the_page_closes() {
        let ran = plan(mark_resolved(
            &context(state(), target(Some("B1\n"), 0)),
            &Answers::new(),
        ));
        let logged = lines(&ran);
        assert_eq!(logged.len(), 2);
        assert!(
            logged[0].starts_with("write ") && logged[0].ends_with("app.rs"),
            "{logged:?}"
        );
        assert_eq!(logged[1], "git add -- src/app.rs");
        assert_eq!(ran.after, [FollowUp::CloseMergePage(String::from("src/app.rs"))]);
    }

    #[test]
    fn merge_view_mark_resolved__asks_first_while_a_conflict_is_left_and_runs_nothing_until_confirmed() {
        let marked = context(state(), target(Some("<<<<<<<\n"), 1));
        let asked = question(mark_resolved(&marked, &Answers::new()));
        assert_eq!(
            asked.title.as_str(),
            "app.rs still has 1 conflict marked by git. Mark it as resolved all the same, with the markers in it?"
        );
        assert!(matches!(asked.kind, QuestionKind::Confirm { ref accept } if accept.as_str() == "Mark as resolved"));
        assert_eq!(asked.default, None);
        let two = context(state(), target(None, 2));
        assert!(
            question(mark_resolved(&two, &Answers::new()))
                .title
                .as_str()
                .contains("has 2 conflicts marked")
        );
        let confirmed = Answers::new().with(MARK_ANYWAY, Answer::Confirmed);
        assert!(matches!(mark_resolved(&marked, &confirmed), Outcome::Run(_)));
    }

    #[test]
    fn merge_view_mark_resolved__a_saved_file_is_only_staged() {
        let ran = plan(mark_resolved(&context(state(), target(None, 0)), &Answers::new()));
        assert_eq!(lines(&ran), ["git add -- src/app.rs"]);
    }
}
