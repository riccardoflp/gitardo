//! `bisect.end`: ends the bisect, back to where it started.

use model::contract::{Answers, Context, Danger, FollowUp, Outcome, Plan, Step, Text, Undo};

use super::bisect_under_way;

/// Ends the bisect with `git bisect reset`, which checks out again what was checked out before it started. Undo checks
/// out again the commit that was being tested, by the reflog, without starting the bisect again.
///
/// Feature: bisect.end
#[must_use]
pub fn end(context: &Context, _answers: &Answers) -> Outcome {
    if let Err(refusal) = bisect_under_way(context) {
        return *refusal;
    }
    Outcome::Run(Plan {
        title: Text::from("End bisect"),
        steps: vec![Step::git(context.repository.folder.clone(), ["bisect", "reset"])],
        danger: Danger::None,
        undo: Undo::ByReflog,
        after: vec![FollowUp::SelectHead],
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answers, Outcome, Target};

    use super::end;
    use crate::bisect::testing::{bisecting, marked};
    use crate::testing::{lines, plan};

    #[test]
    fn bisect_end__runs_git_bisect_reset() {
        let context = bisecting(Some(marked(Some('e'), &['1'], Some(4))), Target::Head);
        let plan = plan(end(&context, &Answers::new()));
        assert_eq!(plan.title.as_str(), "End bisect");
        assert_eq!(lines(&plan), ["git bisect reset"]);
    }

    #[test]
    fn bisect_end__is_refused_without_a_bisect() {
        assert!(matches!(
            end(&bisecting(None, Target::Head), &Answers::new()),
            Outcome::Refuse(_)
        ));
    }
}
