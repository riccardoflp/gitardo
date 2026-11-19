//! What an operation that ran asks of the window once it ended: a page opened, text on the clipboard, a comparison
//! shown.

use model::contract::FollowUp;

use crate::store::{Effect, State};

/// What an operation that ran asks of the window after it: a page opened, text on the clipboard, a comparison shown.
pub(super) fn carry_out(state: &mut State, after: Vec<FollowUp>) -> Vec<Effect> {
    let mut effects = Vec::new();
    for follow_up in after {
        match (follow_up, state.shown_folder().cloned()) {
            (FollowUp::OpenCommitPage(commit), Some(folder)) => {
                effects.extend(crate::store::commit_page::open(state, folder, commit));
            }
            (FollowUp::CopyToClipboard(text), _) => effects.push(Effect::CopyText(text)),
            (FollowUp::OpenUrl(url), _) => effects.push(Effect::OpenUrl(url)),
            (FollowUp::RememberInputs(values), _) => effects.extend(super::remember_values(state, &values)),
            (FollowUp::OpenWindow(folder), _) => effects.push(Effect::OpenWindow(folder)),
            (FollowUp::OpenComparison(target), _) => effects.extend(crate::store::compare::opened(state, *target)),
            (FollowUp::OpenRangeDiff(target), Some(folder)) => {
                effects.extend(crate::store::range_diff::open(state, folder, *target));
            }
            (FollowUp::OpenFileDiff(sides), _) => {
                effects.extend(crate::store::diff_page::open(state, views::diff::file_sides(&sides)));
            }
            (FollowUp::CloseMergePage(path), _) => crate::store::merge_page::close_file(state, &path),
            (FollowUp::OpenReview(files), _) => {
                state.operations.said = None;
                effects.extend(crate::store::review::open(state, Some(files)));
            }
            (FollowUp::CompareInPlace(sides), Some(folder)) => {
                effects.extend(crate::store::compare::in_place::show(state, folder, *sides));
            }
            (FollowUp::ShowHistory(history), _) => {
                effects.extend(crate::store::search::show_history(
                    state,
                    history.map(|history| *history),
                ));
            }
            (FollowUp::SelectCommit(_), _) => effects.extend(crate::store::pages::show_timeline(state)),
            (FollowUp::EditFile(path, line), _) => {
                let target = views::file_view::FileTarget {
                    path,
                    revision: None,
                    line: Some(line),
                };
                effects.extend(crate::store::file_page::open(state, target));
            }
            (blamed @ (FollowUp::ShowBlame(_) | FollowUp::HideBlame(..) | FollowUp::OpenFile(_)), _) => {
                effects.extend(crate::store::blame::carry_out(state, blamed));
            }
            (tasks @ (FollowUp::RunTasks(_) | FollowUp::StopTasks(_)), _) => {
                effects.extend(crate::store::tasks_tab::carry_out(state, tasks));
            }
            (other, _) => effects.extend(crate::store::worktrees::carry_out(state, other)),
        }
    }
    effects
}
