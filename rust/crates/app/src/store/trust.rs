//! The folders the user trusts, as the window gives them to the operations and keeps what the operations decide of
//! them (`trust.is-trusted`, `trust.ask`, `trust.manage`).

use std::path::Path;

use model::contract::{FeatureSettings, FollowUp};

use super::feature::Ended;
use super::{Effect, State};
use crate::persisted::trust::{forget, trust};

/// Gives an operation that runs in a folder whether that folder is trusted, and the folders trusted.
pub(super) fn settings_of(state: &State, folder: &Path, settings: &mut FeatureSettings) {
    settings.trust = model::trust::is_trusted(folder, &state.persisted.trusted_folders);
    settings.trusted_folders.clone_from(&state.persisted.trusted_folders);
}

/// Keeps what an operation that ended decided of the folders trusted, at once: a folder trusted in answer to
/// `trust.ask` is trusted whether its plan succeeded or failed, and before the commands it runs end, which what else
/// follows the plan waits for.
pub(super) fn keep(state: &mut State, ended: &Ended) -> Vec<Effect> {
    let follow_ups = match ended {
        Ended::Ran { after, .. } => after.as_slice(),
        Ended::Failed { on_failure, .. } => on_failure.as_slice(),
        Ended::Refused(..) | Ended::BackedOut => return Vec::new(),
    };
    let mut is_changed = false;
    for follow_up in follow_ups {
        match follow_up {
            FollowUp::TrustFolder(folder) => {
                trust(&mut state.persisted.trusted_folders, folder);
                is_changed = true;
            }
            FollowUp::ForgetTrustedFolders(folders) => {
                forget(&mut state.persisted.trusted_folders, folders);
                is_changed = true;
            }
            _ => {} // The other follow-ups are carried out once the operation's end is said.
        }
    }
    if is_changed {
        vec![Effect::WritePersisted(state.persisted.clone())]
    } else {
        Vec::new()
    }
}
