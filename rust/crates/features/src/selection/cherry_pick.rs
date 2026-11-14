//! `selection.cherry-pick`: applies several selected commits onto what is checked out with one `git cherry-pick`, so
//! that git treats them as one operation to continue, skip or abort.

use model::contract::{
    AnswerKey, Answers, Context, Danger, Outcome, Plan, Query, Refusal, Step, Target, Text, Undo, UndoRecord,
};

use crate::cherry_pick::skip_when_empty;
use crate::history::{current, refuse_under_way};

/// The commits picked, read for their parents.
const PICKED: AnswerKey = AnswerKey("picked");

/// Picks the commits selected, oldest first whatever the order they were clicked in, with `git cherry-pick [-x]
/// <hash>…`; Undo takes them all back at once, to where `HEAD` was before. A merge among them is refused before git
/// runs.
///
/// Feature: selection.cherry-pick
#[must_use]
pub fn cherry_pick(context: &Context, answers: &Answers) -> Outcome {
    if let Some(refusal) = refuse_under_way(context) {
        return refusal;
    }
    let commits = match &context.target {
        Target::Commits(commits) if commits.len() >= 2 => commits,
        _ => return refuse("Select the commits to cherry-pick in the timeline."),
    };
    let Some(head) = context.repository.head.commit.clone() else {
        return refuse("There is nothing to cherry-pick onto before the first commit.");
    };
    let Some(picked) = answers.commits(PICKED) else {
        return Outcome::Read(
            PICKED,
            Query::Commits {
                revisions: commits.iter().map(ToString::to_string).collect(),
            },
        );
    };
    if picked.iter().any(model::commit::Commit::is_merge) {
        return refuse("A merge commit can only be cherry-picked on its own.");
    }
    let mut arguments = vec![String::from("cherry-pick")];
    if context.settings.cherry_pick_record_origin {
        arguments.push(String::from("-x"));
    }
    // The timeline lists the newest first: the oldest is picked first.
    arguments.extend(commits.iter().rev().map(ToString::to_string));
    Outcome::Run(Plan {
        title: Text::from(format!(
            "Cherry-pick {} commits onto {}",
            commits.len(),
            current(context)
        )),
        steps: vec![Step::git(context.repository.folder.clone(), arguments)],
        danger: Danger::None,
        undo: Undo::Journal(UndoRecord::Series { head_before: head }),
        after: Vec::new(),
        on_failure: skip_when_empty(),
    })
}

/// An outcome that refuses, saying why.
fn refuse(message: &str) -> Outcome {
    Outcome::Refuse(Refusal::saying(message))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{PICKED, cherry_pick};
    use crate::testing::{commit, context, lines, plan, state};
    use model::commit::Commit;
    use model::contract::{Answer, Answers, Context, Outcome, Target, Undo, UndoRecord};
    use model::{CommitId, Timestamp};

    fn row(id: CommitId, parents: usize) -> Commit {
        Commit {
            id,
            parents: (0..parents).map(|_| commit('f')).collect(),
            author_name: String::new(),
            author_email: String::new(),
            author_date: Timestamp(0),
            subject: String::new(),
        }
    }

    /// `C`, `B` and `A`, newest first, as the timeline lists them whatever the order they were clicked in.
    fn three() -> (Context, Answers) {
        let selected = vec![commit('c'), commit('b'), commit('1')];
        let picked = selected.iter().map(|id| row(id.clone(), 1)).collect();
        (
            context(state(), Target::Commits(selected)),
            Answers::new().with(PICKED, Answer::Commits(picked)),
        )
    }

    #[test]
    fn selection_cherry_pick__picks_the_oldest_first() {
        let (context, answers) = three();
        let plan = plan(cherry_pick(&context, &answers));
        assert_eq!(
            lines(&plan),
            [format!(
                "git cherry-pick {} {} {}",
                commit('1'),
                commit('b'),
                commit('c')
            )]
        );
        assert_eq!(
            plan.undo,
            Undo::Journal(UndoRecord::Series {
                head_before: commit('a')
            })
        );
    }

    #[test]
    fn selection_cherry_pick__record_origin_adds_x() {
        let (mut context, answers) = three();
        context.settings.cherry_pick_record_origin = true;
        let plan = plan(cherry_pick(&context, &answers));
        assert_eq!(
            lines(&plan),
            [format!(
                "git cherry-pick -x {} {} {}",
                commit('1'),
                commit('b'),
                commit('c')
            )]
        );
    }

    #[test]
    fn selection_cherry_pick__a_merge_among_them_is_refused_before_git_runs() {
        let (context, _) = three();
        let picked = vec![row(commit('c'), 2), row(commit('b'), 1), row(commit('1'), 1)];
        let Outcome::Refuse(refusal) = cherry_pick(&context, &Answers::new().with(PICKED, Answer::Commits(picked)))
        else {
            panic!("a merge is refused")
        };
        assert_eq!(
            refusal.message.as_str(),
            "A merge commit can only be cherry-picked on its own."
        );
    }
}
