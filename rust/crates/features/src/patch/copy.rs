//! `patch.copy`: one commit, or several selected, copied as one patch.

use model::contract::{Answers, Context, Danger, FollowUp, Outcome, Plan, Text, Undo};

use super::made;

/// Puts on the clipboard the patch `git format-patch` makes of the commits chosen, oldest first whatever the order they
/// were chosen in, for `git am` to make them again elsewhere. Nothing changes in the repository.
///
/// Feature: patch.copy
#[must_use]
pub fn copy(context: &Context, answers: &Answers) -> Outcome {
    let made = match made(context, answers) {
        Ok(made) => made,
        Err(outcome) => return *outcome,
    };
    let named = made.named();
    Outcome::Run(Plan {
        title: Text::from(format!("Copy {named} as a patch")),
        steps: Vec::new(),
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Copying a patch changes nothing to undo.")),
        after: vec![
            FollowUp::CopyToClipboard(made.patch.to_owned()),
            FollowUp::Say(Text::from(format!("Copied {named} as a patch."))),
        ],
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answer, Answers, FollowUp, Outcome, Query, Target};

    use super::copy;
    use crate::patch::PATCH;
    use crate::patch::testing::format_patch;
    use crate::testing::{commit, context, plan, state};

    #[test]
    fn patch_copy__reads_the_patch_of_the_commit_then_puts_it_on_the_clipboard() {
        let context = context(state(), Target::Commit(commit('3')));
        assert_eq!(
            copy(&context, &Answers::new()),
            Outcome::Read(
                PATCH,
                Query::FormatPatch {
                    commits: vec![commit('3')]
                }
            )
        );
        let patch = format_patch(&[&commit('3')]);
        let answers = Answers::new().with(PATCH, Answer::Patch(patch.clone()));
        let plan = plan(copy(&context, &answers));
        assert_eq!(plan.steps, []);
        assert_eq!(
            plan.after,
            [
                FollowUp::CopyToClipboard(patch),
                FollowUp::Say(format!("Copied {} as a patch.", commit('3').short()).into()),
            ]
        );
    }

    #[test]
    fn patch_copy__three_commits_say_how_many_and_a_merge_among_them_is_refused() {
        let chosen = vec![commit('3'), commit('1'), commit('2')];
        let context = context(state(), Target::Commits(chosen));
        let patch = format_patch(&[&commit('1'), &commit('2'), &commit('3')]);
        let answers = Answers::new().with(PATCH, Answer::Patch(patch));
        let plan = plan(copy(&context, &answers));
        assert_eq!(plan.title.as_str(), "Copy 3 commits as a patch");
        let short = format_patch(&[&commit('1'), &commit('2')]);
        let Outcome::Refuse(refusal) = copy(&context, &Answers::new().with(PATCH, Answer::Patch(short))) else {
            panic!("a merge among the commits is refused")
        };
        assert_eq!(
            refusal.message.as_str(),
            "Git makes no patch of a merge commit: leave the merges out of the selection."
        );
    }
}
