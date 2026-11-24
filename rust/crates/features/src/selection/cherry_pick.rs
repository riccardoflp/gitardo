//! `selection.cherry-pick`: applies several selected commits onto what is checked out with one `git cherry-pick`, so
//! that git treats them as one operation to continue, skip or abort.

use model::contract::{
    AnswerKey, Answers, Context, Outcome, Plan, Query, Refusal, Step, Target, Text, Undo, UndoRecord,
};

use crate::cherry_pick::skip_when_empty;
use crate::history::{conflicts_danger, current, refuse_under_way};

/// The commits picked, read for their parents.
const PICKED: AnswerKey = AnswerKey("picked");

/// The files the first pick to stop would leave in conflict.
const CONFLICTS: AnswerKey = AnswerKey("conflicts");

/// Picks the commits selected, oldest first whatever the order they were clicked in, with `git cherry-pick [-x]
/// <hash>…`; Undo takes them all back at once, to where `HEAD` was before. A merge among them is refused before git
/// runs. It asks first when the series is predicted to stop on conflicts, each commit picked onto what the one before
/// it made, and names the files.
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
    // The timeline lists the newest first: the oldest is picked first.
    let oldest_first: Vec<String> = commits.iter().rev().map(ToString::to_string).collect();
    let Some(conflicts) = answers.conflicts(CONFLICTS) else {
        return Outcome::Read(CONFLICTS, Query::PredictedSeriesConflicts { commits: oldest_first });
    };
    let mut arguments = vec![String::from("cherry-pick")];
    if context.settings.cherry_pick_record_origin {
        arguments.push(String::from("-x"));
    }
    arguments.extend(oldest_first);
    Outcome::Run(Plan {
        title: Text::from(format!(
            "Cherry-pick {} commits onto {}",
            commits.len(),
            current(context)
        )),
        steps: vec![Step::git(context.repository.folder.clone(), arguments)],
        danger: conflicts_danger(None, conflicts),
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
    use super::{CONFLICTS, PICKED, cherry_pick};
    use crate::testing::{commit, context, lines, plan, state};
    use model::commit::Commit;
    use model::contract::{Answer, Answers, Context, Danger, Outcome, Query, Target, Undo, UndoRecord};
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
            Answers::new()
                .with(PICKED, Answer::Commits(picked))
                .with(CONFLICTS, Answer::Conflicts(Some(Vec::new()))),
        )
    }

    #[test]
    fn selection_cherry_pick__predicts_the_conflicts_of_the_series_oldest_first_and_asks_naming_the_files() {
        let selected = vec![commit('c'), commit('b'), commit('1')];
        let picked: Vec<Commit> = selected.iter().map(|id| row(id.clone(), 1)).collect();
        let selection = context(state(), Target::Commits(selected));
        let answers = Answers::new().with(PICKED, Answer::Commits(picked));
        let Outcome::Read(CONFLICTS, Query::PredictedSeriesConflicts { commits }) = cherry_pick(&selection, &answers)
        else {
            panic!("the conflicts of the series are predicted")
        };
        assert_eq!(
            commits,
            [commit('1'), commit('b'), commit('c')].map(|id| id.to_string())
        );
        let stopping = answers.with(CONFLICTS, Answer::Conflicts(Some(vec![String::from("a.txt")])));
        let asked = plan(cherry_pick(&selection, &stopping));
        assert!(
            matches!(asked.danger, Danger::Confirm(ref reason) if reason.as_str() == "This will stop with conflicts in 1 file:\na.txt"),
            "{:?}",
            asked.danger
        );
        // A git that cannot tell runs the series as one that stops on none.
        let unknown = Answers::new()
            .with(PICKED, Answer::Commits(vec![row(commit('b'), 1), row(commit('1'), 1)]))
            .with(CONFLICTS, Answer::Conflicts(None));
        let two = context(state(), Target::Commits(vec![commit('b'), commit('1')]));
        assert_eq!(plan(cherry_pick(&two, &unknown)).danger, Danger::None);
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
