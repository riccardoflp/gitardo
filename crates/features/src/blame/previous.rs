//! `blame.previous`: opens the file as the commit that last changed a line found it, with the blame it had then.

use model::blame::{BlameOfFile, BlameQuery, FileToOpen};
use model::contract::{AnswerKey, Answers, Context, FollowUp, Outcome, Query, Refusal};

use super::{line_commit, line_of_file, no_step};

/// The key of the blame of the file as it was before.
pub const BEFORE: AnswerKey = AnswerKey("blame before");

/// Opens the file as it was in the commit before the one that last changed the line under the cursor, under the name
/// it had there, read only, with its blame and the cursor on the line of the same number, or the last line. The commit
/// that added the file has nothing before it, and opens nothing.
///
/// Feature: blame.previous
#[must_use]
pub fn previous(context: &Context, answers: &Answers) -> Outcome {
    let commit = match line_commit(context, answers) {
        Ok(commit) => commit,
        Err(outcome) => return *outcome,
    };
    let Some(before) = &commit.previous else {
        let said = format!(
            "{} is the commit that added this file, so there is nothing before it to blame.",
            commit.id.short()
        );
        return Outcome::Refuse(Refusal::saying(&said));
    };
    let Some(lines) = answers.blame(BEFORE) else {
        let query = BlameQuery {
            path: before.path.clone(),
            revision: Some(before.commit.clone()),
            unsaved: None,
            ignores_whitespace: context.settings.blame.ignores_whitespace,
        };
        return Outcome::Read(BEFORE, Query::Blame(Box::new(query)));
    };
    let Some(lines) = lines else {
        let said = format!("Git has no blame of {} as of {}.", before.path, before.commit.short());
        return Outcome::Refuse(Refusal::saying(&said));
    };
    let wanted = line_of_file(context).map_or(1, |line| line.line);
    let file = FileToOpen {
        line: wanted.min(lines.len()).max(1),
        blame: BlameOfFile {
            path: before.path.clone(),
            revision: Some(before.commit.clone()),
            lines: lines.to_vec(),
        },
    };
    no_step(
        format!("Blame {} as of {}", before.path, before.commit.short()),
        FollowUp::OpenFile(Box::new(file)),
    )
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answer, Answers, FollowUp, Outcome, Query};

    use super::{BEFORE, previous};
    use model::blame::ShownBlame;

    use crate::blame::testing::{blamed, on_line};
    use crate::testing::{commit, plan};

    /// What the operation opens, with the blame of the file before given.
    fn opened(path: &str, line: usize) -> (String, Option<model::CommitId>, usize) {
        let context = on_line(line, ShownBlame::Line(Some(blamed('b', Some(('a', path))))));
        let Outcome::Read(BEFORE, Query::Blame(query)) = previous(&context, &Answers::new()) else {
            panic!("reads the blame of the file before");
        };
        assert_eq!((query.path.as_str(), query.revision.clone()), (path, Some(commit('a'))));
        let lines = Answer::Blame(Some(vec![Some(blamed('a', None)), Some(blamed('a', None))]));
        let plan = plan(previous(&context, &Answers::new().with(BEFORE, lines)));
        let [FollowUp::OpenFile(file)] = plan.after.as_slice() else {
            panic!("opens the file before: {plan:?}");
        };
        assert_eq!(file.blame.lines.len(), 2);
        (file.blame.path.clone(), file.blame.revision.clone(), file.line)
    }

    #[test]
    fn blame_previous__opens_the_file_as_of_the_commit_before_with_its_blame() {
        assert_eq!(
            opened("src/app.rs", 2),
            (String::from("src/app.rs"), Some(commit('a')), 2)
        );
        assert_eq!(opened("src/app.rs", 9).2, 2);
    }

    #[test]
    fn blame_previous__opens_a_renamed_file_under_its_old_name() {
        assert_eq!(opened("src/old.rs", 1).0, "src/old.rs");
    }

    #[test]
    fn blame_previous__says_the_commit_that_added_the_file_has_nothing_before_it() {
        let context = on_line(1, ShownBlame::Line(Some(blamed('b', None))));
        let Outcome::Refuse(refusal) = previous(&context, &Answers::new()) else {
            panic!("opens nothing");
        };
        assert_eq!(
            refusal.message.as_str(),
            "bbbbbbb is the commit that added this file, so there is nothing before it to blame."
        );
    }
}
