//! `blame.open-line-commit`: opens the commit that last changed the line under the cursor as a page of its own.

use model::contract::{Answers, Context, FollowUp, Outcome};

use super::{line_commit, no_step};

/// Opens the page of the commit that last changed the line under the cursor of a File page, from the blame the page
/// shows or read for it; a line not committed yet, or one git has no blame for, opens nothing and says so.
///
/// Feature: blame.open-line-commit
#[must_use]
pub fn open_line_commit(context: &Context, answers: &Answers) -> Outcome {
    match line_commit(context, answers) {
        Ok(commit) => no_step(
            format!("Open commit {} as a page", commit.id.short()),
            FollowUp::OpenCommitPage(commit.id.clone()),
        ),
        Err(outcome) => *outcome,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::blame::BlameLine;
    use model::contract::{Answer, Answers, FollowUp, Outcome, Query};

    use super::open_line_commit;
    use crate::blame::BLAME;
    use model::blame::ShownBlame;

    use crate::blame::testing::{blamed, on_line};
    use crate::testing::{commit, plan};

    #[test]
    fn blame_open_line_commit__opens_the_page_of_the_commit_of_the_line() {
        let shown = on_line(2, ShownBlame::Line(Some(blamed('b', None))));
        let shown_plan = plan(open_line_commit(&shown, &Answers::new()));
        assert_eq!(shown_plan.after, [FollowUp::OpenCommitPage(commit('b'))]);
        assert_eq!(shown_plan.steps, []);
        let read = on_line(2, ShownBlame::Hidden);
        let Outcome::Read(BLAME, Query::Blame(query)) = open_line_commit(&read, &Answers::new()) else {
            panic!("reads the blame of the file when the page shows none");
        };
        assert_eq!(query.path, "src/app.rs");
        let lines = Answer::Blame(Some(vec![Some(blamed('a', None)), Some(blamed('b', None))]));
        let read_plan = plan(open_line_commit(&read, &Answers::new().with(BLAME, lines)));
        assert_eq!(read_plan.after, [FollowUp::OpenCommitPage(commit('b'))]);
    }

    #[test]
    fn blame_open_line_commit__says_a_line_typed_since_is_not_committed_yet() {
        let typed = on_line(2, ShownBlame::Line(Some(BlameLine::NotCommitted)));
        let Outcome::Refuse(refusal) = open_line_commit(&typed, &Answers::new()) else {
            panic!("opens nothing");
        };
        assert_eq!(refusal.message.as_str(), "This line is not committed yet.");
        let unknown = on_line(2, ShownBlame::Line(None));
        let Outcome::Refuse(refusal) = open_line_commit(&unknown, &Answers::new()) else {
            panic!("opens nothing");
        };
        assert_eq!(refusal.message.as_str(), "This line is in no commit.");
    }
}
