//! `bisect.start`: starts a bisect from a commit known to be bad, or known to work.

use model::contract::{Answers, Context, Outcome, Refusal};

use super::{bisect_plan, good_or_bad, refuse_while_operation, target_commit};

/// Starts a bisect with `git bisect start`, then marks the commit invoked on bad or good, as the entry chosen says;
/// refused while a bisect or another operation is under way. The banner then asks for the other mark.
///
/// Feature: bisect.start
#[must_use]
pub fn start(context: &Context, answers: &Answers) -> Outcome {
    let commit = match target_commit(context) {
        Ok(commit) => commit,
        Err(refusal) => return *refusal,
    };
    if context.repository.bisect.is_some() {
        return Outcome::Refuse(Refusal::saying("A bisect is under way already."));
    }
    if let Some(refusal) = refuse_while_operation(context) {
        return refusal;
    }
    let mark = match good_or_bad(answers, "Start bisect") {
        Ok(mark) => mark,
        Err(question) => return *question,
    };
    Outcome::Run(bisect_plan(
        context,
        format!("Start bisect: {} is {mark}", commit.short()),
        &[&["bisect", "start"], &["bisect", mark, commit.as_str()]],
        Vec::new(),
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answer, Answers, Outcome, Target};
    use model::state::Operation;

    use super::start;
    use crate::bisect::MARK;
    use crate::bisect::testing::{bisecting, marked};
    use crate::testing::{commit, lines, plan};

    #[test]
    fn bisect_start__this_commit_is_bad_starts_and_marks_it() {
        let context = bisecting(None, Target::Commit(commit('e')));
        let answers = Answers::new().with(MARK, Answer::Choice(String::from("bad")));
        let plan = plan(start(&context, &answers));
        assert_eq!(plan.title.as_str(), "Start bisect: eeeeeee is bad");
        assert_eq!(
            lines(&plan),
            ["git bisect start", &format!("git bisect bad {}", commit('e'))]
        );
    }

    #[test]
    fn bisect_start__is_refused_while_a_rebase_is_stopped_or_a_bisect_is_under_way() {
        let mut context = bisecting(None, Target::Commit(commit('e')));
        let mut state = (*context.repository).clone();
        state.operation = Some(Operation::Rebase);
        context.repository = std::sync::Arc::new(state);
        let answers = Answers::new().with(MARK, Answer::Choice(String::from("good")));
        assert!(matches!(start(&context, &answers), Outcome::Refuse(_)));
        let under_way = bisecting(Some(marked(Some('e'), &[], None)), Target::Commit(commit('a')));
        assert!(matches!(start(&under_way, &answers), Outcome::Refuse(_)));
    }
}
