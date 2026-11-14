//! `bisect.mark`: while a bisect is under way, marks any commit of the timeline good or bad.

use model::contract::{Answers, Context, FollowUp, Outcome};

use super::{bisect_plan, bisect_under_way, good_or_bad, refuse_while_operation, target_commit};

/// Marks the commit invoked on with `git bisect good <hash>` or `git bisect bad <hash>`; once a good and a bad one are
/// marked git checks out a commit between them, and when one commit is left the window says it is the first bad one.
///
/// Feature: bisect.mark
#[must_use]
pub fn mark(context: &Context, answers: &Answers) -> Outcome {
    let commit = match target_commit(context) {
        Ok(commit) => commit,
        Err(refusal) => return *refusal,
    };
    if let Err(refusal) = bisect_under_way(context) {
        return *refusal;
    }
    if let Some(refusal) = refuse_while_operation(context) {
        return refusal;
    }
    let mark = match good_or_bad(answers, "Bisect") {
        Ok(mark) => mark,
        Err(question) => return *question,
    };
    Outcome::Run(bisect_plan(
        context,
        format!("Bisect: {} is {mark}", commit.short()),
        &[&["bisect", mark, commit.as_str()]],
        vec![FollowUp::CheckBisect],
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answer, Answers, FollowUp, Outcome, Target};

    use super::mark;
    use crate::bisect::MARK;
    use crate::bisect::testing::{bisecting, marked};
    use crate::testing::{commit, lines, plan};

    #[test]
    fn bisect_mark__marking_a_commit_good_runs_git_bisect_good_with_it() {
        let context = bisecting(Some(marked(Some('e'), &[], None)), Target::Commit(commit('1')));
        let answers = Answers::new().with(MARK, Answer::Choice(String::from("good")));
        let plan = plan(mark(&context, &answers));
        assert_eq!(plan.title.as_str(), "Bisect: 1111111 is good");
        assert_eq!(lines(&plan), [format!("git bisect good {}", commit('1'))]);
        assert!(matches!(
            mark(&bisecting(None, Target::Commit(commit('1'))), &answers),
            Outcome::Refuse(_)
        ));
    }

    #[test]
    fn bisect_mark__asks_the_window_to_say_the_first_bad_commit_once_found() {
        let context = bisecting(Some(marked(Some('e'), &['1'], Some(2))), Target::Commit(commit('2')));
        let answers = Answers::new().with(MARK, Answer::Choice(String::from("bad")));
        assert_eq!(plan(mark(&context, &answers)).after, [FollowUp::CheckBisect]);
    }
}
