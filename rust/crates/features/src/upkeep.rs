//! The upkeep of a repository, from the menu of its row in **Repositories**: **Clean up…** with `git gc`
//! (`upkeep.clean-up`), git's commit-graph file written (`upkeep.write-commit-graph`), and git's scheduled maintenance
//! started and stopped for it (`upkeep.start-background`, `upkeep.stop-background`).

use model::branch::repository_name;
use model::contract::{AnswerKey, Answers, Context, Danger, FollowUp, Outcome, Plan, Query, Refusal, Step, Text, Undo};
use model::upkeep::{RepositorySize, size_sentence, size_text};

/// The room the repository takes, read before the question (`upkeep.read-size`).
pub const SIZE: AnswerKey = AnswerKey("size");

/// What **Clean up…** warns of.
const DELETED: &str = "Commits that no branch, tag or reflog has reached for two weeks, and reflog entries older than \
                       90 days, are deleted for good, as git's settings of gc say.";
/// What **Look after it in the background…** warns of.
const SCHEDULED: &str = "Git is scheduled to run in the background on this computer, every hour, for each repository \
                         registered.";

/// Packs the loose objects of the repository and deletes those nothing has reached for a long time, with `git gc`,
/// after saying the room it takes.
///
/// Feature: upkeep.clean-up
#[must_use]
pub fn clean_up(context: &Context, answers: &Answers) -> Outcome {
    let Some(size) = answers.names(SIZE) else {
        return Outcome::Read(SIZE, Query::RepositorySize);
    };
    let size = RepositorySize::from_names(size);
    let name = repository_name(&context.repository);
    Outcome::Run(Plan {
        title: Text::from(format!("Clean up {name}")),
        steps: vec![Step::git(context.repository.folder.clone(), ["gc"])],
        danger: Danger::Confirm(Text::from(format!("{}\n\n{DELETED}", size_sentence(&name, &size)))),
        undo: Undo::Impossible(Text::from("Cannot be undone: what git deletes is not kept.")),
        // The size after is not read once git ended (Q1455): the message says the size before.
        after: vec![FollowUp::Say(Text::from(format!(
            "Cleaned up {name}, which took {}.",
            size_text(size.total_kib())
        )))],
        on_failure: Vec::new(),
    })
}

/// Writes git's commit-graph file of the repository, with the paths each commit changed, so that the history, a search
/// and the history of a file are read about twice as fast (D88). Git's configuration is not changed; the file is git's
/// own cache of the commits, which `git gc` writes again.
///
/// Feature: upkeep.write-commit-graph
#[must_use]
pub fn write_commit_graph(context: &Context, _answers: &Answers) -> Outcome {
    let graph = context.repository.commit_graph;
    if graph.is_ignored {
        return Outcome::Refuse(Refusal::saying(
            "Git does not read a commit-graph file in this repository: core.commitGraph is false.",
        ));
    }
    if graph.has_file {
        return Outcome::Refuse(Refusal::saying(
            "Git keeps a commit-graph file of this repository already.",
        ));
    }
    let name = repository_name(&context.repository);
    Outcome::Run(Plan {
        title: Text::from(format!("Speed up reading the history of {name}")),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["commit-graph", "write", "--reachable", "--changed-paths"],
        )],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "Not needed: deleting the file leaves the repository as it was.",
        )),
        after: vec![FollowUp::Say(Text::from(format!(
            "Wrote git's commit-graph file of {name}."
        )))],
        on_failure: Vec::new(),
    })
}

/// Registers the repository for git's scheduled maintenance, with `git maintenance start`, which schedules git in the
/// system's scheduler when it is not scheduled yet.
///
/// Feature: upkeep.start-background
#[must_use]
pub fn start_background(context: &Context, _answers: &Answers) -> Outcome {
    if context.repository.is_looked_after {
        return Outcome::Refuse(Refusal::saying(
            "Git looks after this repository in the background already.",
        ));
    }
    Outcome::Run(Plan {
        title: Text::from(format!(
            "Look after {} in the background",
            repository_name(&context.repository)
        )),
        steps: vec![Step::git(context.repository.folder.clone(), ["maintenance", "start"])],
        danger: Danger::Confirm(Text::from(SCHEDULED)),
        undo: Undo::Impossible(Text::from(
            "Not taken back by Undo: Look after it in the background, ticked, stops it.",
        )),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

/// Takes the repository out of those git's scheduled maintenance looks after, with `git maintenance unregister`; the
/// schedule of the system stays for the others (Q1222).
///
/// Feature: upkeep.stop-background
#[must_use]
pub fn stop_background(context: &Context, _answers: &Answers) -> Outcome {
    if !context.repository.is_looked_after {
        return Outcome::Refuse(Refusal::saying(
            "Git does not look after this repository in the background.",
        ));
    }
    Outcome::Run(Plan {
        title: Text::from(format!(
            "Stop looking after {} in the background",
            repository_name(&context.repository)
        )),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["maintenance", "unregister"],
        )],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "Not taken back by Undo: Look after it in the background… registers it again.",
        )),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answer, Answers, Danger, Outcome, Query, Target};
    use model::upkeep::RepositorySize;

    use super::{SIZE, clean_up, start_background, stop_background, write_commit_graph};
    use crate::testing::{context, lines, plan, state};

    fn sized() -> Answers {
        let size = RepositorySize {
            loose: 29,
            loose_kib: 1,
            packed: 1200,
            packs: 2,
            pack_kib: 5120,
            ..RepositorySize::default()
        };
        Answers::new().with(SIZE, Answer::Names(size.to_names()))
    }

    #[test]
    fn upkeep_clean_up__it_reads_the_size_then_asks_with_it_and_the_warning_then_runs_gc() {
        let context = context(state(), Target::Head);
        assert_eq!(
            clean_up(&context, &Answers::new()),
            Outcome::Read(SIZE, Query::RepositorySize)
        );
        let plan = plan(clean_up(&context, &sized()));
        assert_eq!(lines(&plan), ["git gc"]);
        let Danger::Confirm(said) = &plan.danger else {
            panic!("it asks first")
        };
        assert!(
            said.as_str()
                .contains("takes 5.0 MB: 5.0 MB in 2 packs, and 29 loose objects in 1 KB."),
            "{said}"
        );
        assert!(said.as_str().contains("deleted for good"), "{said}");
    }

    #[test]
    fn upkeep_start_background__it_asks_with_the_warning_and_runs_maintenance_start() {
        let context = context(state(), Target::Head);
        let plan = plan(start_background(&context, &Answers::new()));
        assert_eq!(lines(&plan), ["git maintenance start"]);
        assert!(matches!(&plan.danger, Danger::Confirm(said) if said.as_str().contains("every hour")));
        let mut looked_after = state();
        looked_after.is_looked_after = true;
        let context = crate::testing::context(looked_after, Target::Head);
        assert!(matches!(
            start_background(&context, &Answers::new()),
            Outcome::Refuse(_)
        ));
    }

    #[test]
    fn upkeep_stop_background__it_runs_unregister_and_never_maintenance_stop() {
        let mut looked_after = state();
        looked_after.is_looked_after = true;
        let context = context(looked_after, Target::Head);
        let plan = plan(stop_background(&context, &Answers::new()));
        assert_eq!(lines(&plan), ["git maintenance unregister"]);
        assert_eq!(plan.danger, Danger::None);
        let context = crate::testing::context(state(), Target::Head);
        assert!(matches!(stop_background(&context, &Answers::new()), Outcome::Refuse(_)));
    }

    #[test]
    fn upkeep_write_commit_graph__it_writes_the_file_with_the_changed_paths_and_says_so() {
        let context = context(state(), Target::Head);
        let plan = plan(write_commit_graph(&context, &Answers::new()));
        assert_eq!(lines(&plan), ["git commit-graph write --reachable --changed-paths"]);
        assert_eq!(plan.danger, Danger::None);
        let said = format!("{:?}", plan.after);
        assert!(said.contains("Wrote git's commit-graph file of"), "{said}");
    }

    #[test]
    fn upkeep_write_commit_graph__it_is_refused_where_git_keeps_the_file_or_does_not_read_one() {
        let mut written = state();
        written.commit_graph.has_file = true;
        let context = crate::testing::context(written, Target::Head);
        assert!(matches!(
            write_commit_graph(&context, &Answers::new()),
            Outcome::Refuse(_)
        ));
        let mut ignored = state();
        ignored.commit_graph.is_ignored = true;
        let context = crate::testing::context(ignored, Target::Head);
        assert!(matches!(
            write_commit_graph(&context, &Answers::new()),
            Outcome::Refuse(_)
        ));
    }
}
