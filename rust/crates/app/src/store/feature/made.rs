//! The operations that make a repository from the window, which run with no repository (`repo.clone`, `repo.init`),
//! and what follows them: the folder of the clone remembered, the repository made opened in a tab and shown.

use std::sync::Arc;

use model::contract::{Answer, Answers, FeatureId, FollowUp, Target};
use model::state::RepositoryState;

use super::super::{Effect, State};

/// The operations that make a repository, which run with no repository: their steps run in the folder chosen
/// (`repo.clone`, `repo.init`, Q1051).
pub(in crate::store) const NEW_REPOSITORY: [FeatureId; 2] = [FeatureId("repo.clone"), FeatureId("repo.init")];

/// Starts an operation that makes a repository, given the context of no repository: the folder of the repository shown,
/// or of the first folder of the workspace, else none, which the window takes for the user's home folder. A clone
/// starts its dialog in the folder the last clone went in.
pub(in crate::store) fn run(
    state: &mut State,
    (feature, target): (FeatureId, Target),
    mut preset: Answers,
) -> Vec<Effect> {
    let folder = state
        .shown_folder()
        .cloned()
        .or_else(|| {
            state
                .workspace
                .as_ref()
                .and_then(|workspace| workspace.folders.first().cloned())
        })
        .unwrap_or_default();
    if let Some(remembered) = state.persisted.clone_folder.clone() {
        preset.insert(features::repo::START_FOLDER, Answer::Path(remembered));
    }
    let repository = Arc::new(RepositoryState {
        folder: folder.clone(),
        ..RepositoryState::default()
    });
    super::run_in(state, (folder, repository), (feature, target), preset)
}

/// What follows an operation that made a repository: the folder that held a clone kept as `cloneFolder`, and the
/// repository opened.
pub(in crate::store) fn carry_out(state: &mut State, follow_up: FollowUp) -> Vec<Effect> {
    match follow_up {
        FollowUp::RememberCloneFolder(folder) => {
            state.persisted.clone_folder = Some(folder);
            vec![Effect::WritePersisted(state.persisted.clone())]
        }
        FollowUp::OpenRepository(folder) => super::super::workspace::open_made(state, folder),
        _ => Vec::new(), // The others are carried out elsewhere.
    }
}

/// **Create a repository here**, on the timeline of a workspace with no repository: `repo.init` in the first folder of
/// the workspace, which it does not ask (Q1059).
pub(crate) fn create_repository_here(state: &mut State) -> Vec<Effect> {
    let Some(folder) = state
        .workspace
        .as_ref()
        .and_then(|workspace| workspace.folders.first().cloned())
    else {
        return Vec::new();
    };
    let preset = Answers::new().with(features::repo::FOLDER, Answer::Path(folder));
    let intent = views::feature::FeatureIntent::RunWith(FeatureId("repo.init"), Target::Head, preset);
    super::reduce(state, intent)
}
