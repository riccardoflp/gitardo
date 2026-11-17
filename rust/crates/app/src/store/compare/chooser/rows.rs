//! The sides the comparison chooser offers (`compare.chooser`): those of `compare.side-choices` under their headings,
//! and the last commits of the branch checked out, with the dates and the subjects the timeline has read.

use std::collections::HashMap;
use std::path::PathBuf;

use model::CommitId;
use model::commit::Commit;
use model::compare::{
    CompareSide, PickItem, Side, SideChoice, SideChoicesInput, SideInput, follow_sides, side_choices,
};
use model::graph::Row;
use model::refs::References;
use model::state::RepositoryState;
use views::compare::{ChooserGroup, ChooserSide};

use crate::persisted::same_folder;
use crate::store::state::State;

/// How many of the last commits of the branch checked out the chooser offers.
const RECENT_COMMITS: usize = 20;

/// The sides the chooser offers in a repository: what is checked out and the working tree, the branches, the remote
/// branches and the tags, then the last commits of the branch checked out; each with the date and the subject of its
/// commit when the timeline has read it.
pub(super) fn rows_of(state: &State, (folder, repository): (&PathBuf, &RepositoryState)) -> Vec<ChooserSide> {
    let is_listed = state
        .timeline
        .folder
        .as_ref()
        .is_some_and(|listed| same_folder(listed, folder));
    let listed: &[Row] = if is_listed { state.timeline.read_rows() } else { &[] };
    let commits: HashMap<&CommitId, &Commit> = listed
        .iter()
        .filter_map(|row| match row {
            Row::Commit(commit) => Some((&commit.id, commit)),
            Row::Stash { .. } | Row::Uncommitted { .. } => None,
        })
        .collect();
    let references = References::of(repository);
    let items = side_choices(&SideChoicesInput {
        references: &references,
        head: repository.head.commit.as_ref(),
        branch: repository.checked_out_branch(),
        chosen: None,
        may_offer_working_tree: true,
        recent: None,
    });
    let mut rows: Vec<ChooserSide> = items.iter().filter_map(|item| offered_side(item, &commits)).collect();
    let mut at = repository.head.commit.as_ref();
    for _ in 0..RECENT_COMMITS {
        let Some(commit) = at.and_then(|at| commits.get(at)) else {
            break;
        };
        rows.push(ChooserSide {
            group: ChooserGroup::RecentCommits,
            side: CompareSide {
                name: commit.id.short().to_owned(),
                side: Side::Commit(commit.id.clone()),
                is_followed: false,
                is_gone: false,
            },
            subject: commit.subject.clone(),
            date: Some(commit.author_date.0),
        });
        at = commit.parents.first();
    }
    rows
}

/// The side of an item of `compare.side-choices`, under its heading; none for what the chooser shows otherwise.
fn offered_side(item: &PickItem, commits: &HashMap<&CommitId, &Commit>) -> Option<ChooserSide> {
    let group = match item.group {
        None => ChooserGroup::CheckedOut,
        Some("Branches") => ChooserGroup::Branches,
        Some("Remote branches") => ChooserGroup::RemoteBranches,
        Some("Tags") => ChooserGroup::Tags,
        Some(_) => return None,
    };
    match &item.choice {
        SideChoice::WorkingTree => Some(ChooserSide {
            group,
            side: CompareSide::working_tree(),
            subject: String::new(),
            date: None,
        }),
        SideChoice::Named { name, commit } => Some(ChooserSide {
            group,
            side: follow_sides(&SideInput {
                name,
                chosen: &Side::Commit(commit.clone()),
                is_followed: None,
                resolved: Some(commit),
            }),
            subject: String::new(),
            date: commits.get(commit).map(|known| known.author_date.0),
        }),
        SideChoice::Recent(_) | SideChoice::EnterRevision => None,
    }
}
