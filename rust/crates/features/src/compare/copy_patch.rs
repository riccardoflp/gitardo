//! `compare.copy-patch`: the changes of a comparison, as listed, copied as a patch that `git apply` takes.

use model::compare::ComparisonPart;
use model::contract::{AnswerKey, Answers, Context, FollowUp, Outcome, Query, Refusal, Target, Text};

use super::no_step;

/// The key of the patch read.
const PATCH: AnswerKey = AnswerKey("patch");

/// Puts on the clipboard the changes of a comparison as a patch, binary files included, in the mode and narrowed to
/// the path of the page, and says how many files it has. Nothing changes in the repository.
///
/// Feature: compare.copy-patch
#[must_use]
pub fn copy_patch(context: &Context, answers: &Answers) -> Outcome {
    let Target::Comparison(target) = &context.target else {
        return Outcome::Refuse(Refusal::saying("Open a comparison first."));
    };
    let Some(patch) = answers.contents(PATCH) else {
        let mut whole = target.as_ref().clone();
        whole.part = ComparisonPart::Whole;
        return Outcome::Read(PATCH, Query::ComparePatch(Box::new(whole)));
    };
    let patch = String::from_utf8_lossy(patch).into_owned();
    let files = patch.lines().filter(|line| line.starts_with("diff --git ")).count();
    let title = format!("Copy {} with {} as a patch", target.first.name, target.second.name);
    if files == 0 {
        return Outcome::Run(no_step(
            title,
            vec![FollowUp::Say(Text::from("There are no changes to copy."))],
        ));
    }
    let counted = if files == 1 {
        String::from("1 file")
    } else {
        format!("{files} files")
    };
    Outcome::Run(no_step(
        title,
        vec![
            FollowUp::CopyToClipboard(patch),
            FollowUp::Say(Text::from(format!("Copied the changes of {counted} as a patch."))),
        ],
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::compare::ComparisonPart;
    use model::contract::{Answer, Answers, FollowUp, Outcome, Query, Target};

    use super::{PATCH, copy_patch};
    use crate::compare::testing::{branch, compared};
    use crate::testing::{context, plan, state};

    #[test]
    fn compare_copy_patch__reads_the_patch_of_the_comparison_and_counts_its_files() {
        let target = compared(branch("main", 'a'), branch("feature", 'b'), ComparisonPart::Whole);
        let context = context(state(), Target::Comparison(Box::new(target.clone())));
        assert_eq!(
            copy_patch(&context, &Answers::new()),
            Outcome::Read(PATCH, Query::ComparePatch(Box::new(target)))
        );
        let patch = "diff --git a/a b/a\n+a\ndiff --git a/b b/b\n+b\n";
        let answers = Answers::new().with(PATCH, Answer::Contents(patch.as_bytes().to_vec()));
        let plan = plan(copy_patch(&context, &answers));
        assert_eq!(
            plan.after,
            [
                FollowUp::CopyToClipboard(patch.to_owned()),
                FollowUp::Say("Copied the changes of 2 files as a patch.".into()),
            ]
        );
    }

    #[test]
    fn compare_copy_patch__with_nothing_to_copy_says_so() {
        let target = compared(branch("main", 'a'), branch("feature", 'a'), ComparisonPart::Whole);
        let context = context(state(), Target::Comparison(Box::new(target)));
        let answers = Answers::new().with(PATCH, Answer::Contents(Vec::new()));
        assert_eq!(
            plan(copy_patch(&context, &answers)).after,
            [FollowUp::Say("There are no changes to copy.".into())]
        );
    }
}
