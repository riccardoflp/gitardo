//! The history of a file, of some lines or of a folder (`docs/features/11-history-and-blame.md`): the timeline
//! narrowed to the commits that changed them, until **Show all commits**. Nothing changes in the repository, so each
//! plan has no step and nothing to undo.

mod file;
mod folder;
mod lines;

pub use file::file;
pub use folder::folder;
pub use lines::lines;

use model::contract::{Context, Danger, FollowUp, Plan, Text, Undo};
use model::file_history::HistoryOf;

/// Why there is nothing to undo.
const NOTHING_TO_UNDO: &str = "Showing a history changes nothing to undo: Show all commits takes the narrowing away.";

/// The path of a file or a folder on disk from the top of the working tree of the repository, with `/`: empty for the
/// top itself, none when the repository does not hold it.
fn path_in_repository(context: &Context, path: &std::path::Path) -> Option<String> {
    model::repo::path_in_folder(path, &context.repository.folder)
}

/// The plan that narrows the timeline, or shows it whole when there is nothing to narrow it to.
fn shows(title: String, history: Option<HistoryOf>) -> Plan {
    Plan {
        title: Text::from(title),
        steps: Vec::new(),
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(NOTHING_TO_UNDO)),
        after: vec![FollowUp::ShowHistory(history.map(Box::new))],
        on_failure: Vec::new(),
    }
}

/// The name of a file without its folders.
fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

#[cfg(test)]
pub(crate) mod testing {
    use std::path::PathBuf;

    use model::contract::{FollowUp, Outcome};
    use model::file_history::HistoryOf;

    use crate::testing::plan;

    /// A file or a folder of the test repository, `repository/`, by its whole path.
    pub(crate) fn on_disk(path: &str) -> PathBuf {
        let mut whole = PathBuf::from("repository");
        whole.extend(path.split('/').filter(|part| !part.is_empty()));
        whole
    }

    /// What the plan of an outcome narrows the timeline to; none for no narrowing.
    pub(crate) fn shown(outcome: Outcome) -> Option<HistoryOf> {
        let plan = plan(outcome);
        assert_eq!(plan.steps, []);
        match plan.after.as_slice() {
            [FollowUp::ShowHistory(history)] => history.clone().map(|history| *history),
            other => panic!("shows a history: {other:?}"),
        }
    }
}
