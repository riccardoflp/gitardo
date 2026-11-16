//! The worktrees in the window (`docs/features/12-worktrees.md`): another worktree shown in this window and going back
//! to the window's own folder (`worktree.show-in-gitardo`, `worktree.banner`), a new window opened on a worktree, and
//! the operations offered once one ran or failed.

use std::path::{Path, PathBuf};

use model::contract::{FeatureId, FollowUp, OfferedRun, WorktreeSettings};

use super::state::RepositoryRead;
use super::{Effect, State, workspace};
use crate::persisted::same_folder;

/// What the window keeps of the worktrees.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Worktrees {
    /// The folder of the window while the views show another worktree; none while they show the window's own. It is
    /// not remembered: the next time the window opens, the views show its own folder.
    pub window_folder: Option<PathBuf>,
    /// Whether **Choose What It Gets…** was offered once a worktree was made, which it is the first time only.
    pub setup_offered: bool,
}

/// What the operations of worktrees are told of the window: the folder of the window while another worktree is shown,
/// and whether **Choose What It Gets…** was offered already.
pub(super) fn settings_of(state: &State, settings: &mut WorktreeSettings) {
    settings.window_folder.clone_from(&state.worktrees.window_folder);
    settings.setup_offered = state.worktrees.setup_offered;
}

/// What the menus and the list of the worktrees are told of the window.
pub(super) fn window_of(state: &State) -> views::worktree::WorktreeWindow<'_> {
    let features = &state.settings.features;
    views::worktree::WorktreeWindow {
        folder: state.worktrees.window_folder.as_deref(),
        has_editor: !features.external_editor.trim().is_empty(),
        setting: &features.worktrees.folder,
    }
}

/// The follow-ups of an operation that the window carries out for the worktrees: another worktree shown in this one,
/// and noting that **Choose What It Gets…** was offered.
pub(super) fn carry_out(state: &mut State, follow_up: FollowUp) -> Vec<Effect> {
    match follow_up {
        FollowUp::ShowFolder(folder) => show(state, &folder),
        FollowUp::OfferRun(offer) => {
            if offer.feature == FeatureId("worktree-setup.page") {
                state.worktrees.setup_offered = true;
            }
            Vec::new()
        }
        _ => Vec::new(), // The other follow-ups are carried out elsewhere.
    }
}

/// The operations offered among follow-ups, each with its button.
pub(super) fn offered(follow_ups: &[FollowUp]) -> Vec<OfferedRun> {
    follow_ups
        .iter()
        .filter_map(|follow_up| match follow_up {
            FollowUp::OfferRun(offer) => Some((**offer).clone()),
            _ => None, // Only these have a button.
        })
        .collect()
}

/// Points the views at another worktree of the repository or at a submodule, keeping the folder of the window to go
/// back to; the folder of the window itself goes back. In a tab, each keeps what it showed apart from the others
/// (`repo.group-tabs`). Nothing while an operation runs.
pub fn show(state: &mut State, folder: &Path) -> Vec<Effect> {
    if state.is_operation_running {
        return Vec::new();
    }
    if state.tabs.shown.is_some() {
        return super::tabs::show_member(state, folder);
    }
    let Some(shown) = state.shown_folder().cloned() else {
        return Vec::new();
    };
    let window = state.worktrees.window_folder.take().unwrap_or(shown);
    if !same_folder(&window, folder) {
        state.worktrees.window_folder = Some(window);
    }
    workspace::show_folder(state, folder)
}

/// The banner of the worktree or the submodule shown while it is not the folder of the window (`worktree.banner`). The
/// folder shown is a submodule when the worktrees it lists do not have the folder of the window; a submodule is named
/// by its path from the window's folder.
pub(super) fn banner_state<'a>(
    state: &'a State,
    read: &'a RepositoryRead,
) -> Option<Box<views::worktree::BannerState<'a>>> {
    let window = state.worktrees.window_folder.as_deref()?;
    let name_of = |folder: &Path| {
        folder.file_name().map_or_else(
            || folder.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        )
    };
    let worktrees = &read.state.worktrees;
    let is_submodule = !worktrees.iter().any(|worktree| same_folder(&worktree.folder, window));
    let label = if is_submodule {
        read.folder.strip_prefix(window).map_or_else(
            |_| name_of(&read.folder),
            |path| {
                path.iter()
                    .map(|part| part.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/")
            },
        )
    } else {
        worktrees.iter().find(|worktree| worktree.is_current).map_or_else(
            || name_of(&read.folder),
            |worktree| views::worktree::label(&read.state, window_of(state), worktree),
        )
    };
    Some(Box::new(views::worktree::BannerState {
        label,
        is_submodule,
        branch: read.state.checked_out_branch().map(str::to_owned),
        folder: &read.folder,
        window_name: name_of(window),
        window_folder: window,
        is_running: state.is_operation_running,
    }))
}
