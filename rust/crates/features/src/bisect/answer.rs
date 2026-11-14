//! `bisect.answer`: the banner's **Good**, **Bad** and **Skip**, which mark the commit git checked out to test.

use model::contract::{AnswerKey, Answers, Context, FollowUp, Outcome, Refusal};

use super::{bisect_plan, bisect_under_way};

/// What is answered for the commit being tested: `good`, `bad` or `skip`, as the button of the banner says.
pub const ANSWER: AnswerKey = AnswerKey("answer");

/// Marks the commit checked out with `git bisect good`, `git bisect bad` or `git bisect skip`; git then checks out the
/// next one to test, and when one commit is left the window says it is the first bad one.
///
/// Feature: bisect.answer
#[must_use]
pub fn answer(context: &Context, answers: &Answers) -> Outcome {
    let bisect = match bisect_under_way(context) {
        Ok(bisect) => bisect,
        Err(refusal) => return *refusal,
    };
    let (word, said) = match answers.choice(ANSWER) {
        Some("good") => ("good", "good"),
        Some("bad") => ("bad", "bad"),
        Some("skip") => ("skip", "skipped"),
        _ => return Outcome::Refuse(Refusal::saying("Answer good, bad or skip.")),
    };
    if bisect.remaining == Some(1) {
        return Outcome::Refuse(Refusal::saying("The bisect found the first bad commit already."));
    }
    Outcome::Run(bisect_plan(
        context,
        format!("Bisect: this commit is {said}"),
        &[&["bisect", word]],
        vec![FollowUp::CheckBisect],
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answer, Answers, FollowUp, Outcome, Target};

    use super::{ANSWER, answer};
    use crate::bisect::testing::{bisecting, marked};
    use crate::testing::{lines, plan};

    fn answered(word: &str) -> Answers {
        Answers::new().with(ANSWER, Answer::Choice(word.to_owned()))
    }

    #[test]
    fn bisect_answer__good_runs_git_bisect_good_on_the_commit_checked_out() {
        let context = bisecting(Some(marked(Some('e'), &['1'], Some(4))), Target::Head);
        let plan = plan(answer(&context, &answered("good")));
        assert_eq!(plan.title.as_str(), "Bisect: this commit is good");
        assert_eq!(lines(&plan), ["git bisect good"]);
    }

    #[test]
    fn bisect_answer__skip_runs_git_bisect_skip_and_is_refused_without_a_bisect() {
        let context = bisecting(Some(marked(Some('e'), &['1'], Some(4))), Target::Head);
        let plan = plan(answer(&context, &answered("skip")));
        assert_eq!(plan.title.as_str(), "Bisect: this commit is skipped");
        assert_eq!(lines(&plan), ["git bisect skip"]);
        let Outcome::Refuse(refusal) = answer(&bisecting(None, Target::Head), &answered("skip")) else {
            panic!("refused without a bisect")
        };
        assert_eq!(refusal.message.as_str(), "No bisect is in progress.");
    }

    #[test]
    fn bisect_answer__asks_the_window_to_say_the_first_bad_commit_once_found() {
        let context = bisecting(Some(marked(Some('e'), &['1'], Some(2))), Target::Head);
        assert_eq!(plan(answer(&context, &answered("bad"))).after, [FollowUp::CheckBisect]);
    }
}
