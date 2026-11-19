//! The page of what a rewrite changed as the store keeps it (`range-diff.page`): what is compared, the pairs read on a
//! worker, the pairs opened and the one under the keyboard. There is one such page: opening another puts it in place.

use std::path::PathBuf;

use git::read::range_diff::RangeDiffRequest;
use model::range_diff::{RangeDiff, RangeDiffTarget};
use views::page::Page;
use views::range_diff::{RangeDiffIntent, RangeDiffRead, RewriteState};

use super::message::{Effect, JobId, JobKind};
use super::state::State;

/// How far the pairs of the page are read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PairsRead {
    /// Not read yet.
    Comparing,
    /// Read.
    Read(Box<RangeDiff>),
    /// The read failed: what git said.
    Failed(String),
}

/// The page of what a rewrite changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RangeDiffPage {
    /// The folder of its repository, which the commits it opens are of.
    pub folder: PathBuf,
    /// What is compared.
    pub target: RangeDiffTarget,
    /// Its name in its tab and its title row: `feature · what changed`.
    pub title: String,
    /// How far the pairs are read.
    pub read: PairsRead,
    /// The read running.
    pub job: Option<JobId>,
    /// The pairs opened, by their place.
    pub opened: Vec<usize>,
    /// The pair under the keyboard.
    pub lit: Option<usize>,
}

/// Opens the page of what a rewrite changed in the repository shown, in place of the one there was, and reads its
/// pairs on a worker (`FollowUp::OpenRangeDiff`).
pub fn open(state: &mut State, folder: PathBuf, target: RangeDiffTarget) -> Vec<Effect> {
    let mut effects = cancel(state);
    let request = RangeDiffRequest {
        ranges: target.ranges.clone(),
    };
    let job = super::workspace::start_job(
        state,
        JobKind::ReadRangeDiff {
            folder: folder.clone(),
            request,
        },
    );
    state.range_diff = Some(RangeDiffPage {
        folder,
        title: views::text::range_diff::title(&target.branch),
        target,
        read: PairsRead::Comparing,
        job: Some(job.id),
        opened: Vec::new(),
        lit: None,
    });
    effects.push(Effect::Start(job));
    super::pages::show_page(state, Page::RangeDiff);
    effects
}

/// Cancels the read of the page there was.
fn cancel(state: &mut State) -> Vec<Effect> {
    let Some(job) = state.range_diff.as_mut().and_then(|page| page.job.take()) else {
        return Vec::new();
    };
    let _cancelled = state.jobs.remove(&job);
    vec![Effect::Cancel(job)]
}

/// Takes the pairs read.
pub fn pairs_read(state: &mut State, job: JobId, result: Result<RangeDiff, String>) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    let Some(page) = state.range_diff.as_mut().filter(|page| page.job == Some(job)) else {
        return Vec::new();
    };
    page.job = None;
    page.read = match result {
        Ok(read) => PairsRead::Read(Box::new(read)),
        Err(said) => PairsRead::Failed(said),
    };
    Vec::new()
}

/// What the page asks: a pair opened or closed, the keyboard moved, a commit opened.
pub fn reduce(state: &mut State, intent: RangeDiffIntent) -> Vec<Effect> {
    let Some(page) = state.range_diff.as_mut() else {
        return Vec::new();
    };
    match intent {
        RangeDiffIntent::Toggle(index) => {
            page.lit = Some(index);
            let is_changed = matches!(&page.read, PairsRead::Read(read)
                if read.pairs.get(index).is_some_and(|pair| pair.kind == model::range_diff::PairKind::Changed));
            if let Some(place) = page.opened.iter().position(|opened| *opened == index) {
                let _closed = page.opened.remove(place);
            } else if is_changed {
                page.opened.push(index);
            }
            Vec::new()
        }
        RangeDiffIntent::Light(index) => {
            page.lit = Some(index);
            Vec::new()
        }
        RangeDiffIntent::OpenCommit(commit) => {
            let folder = page.folder.clone();
            super::commit_page::open(state, folder, commit)
        }
    }
}

/// What the page shows, while it is open.
#[must_use]
pub fn page_state(state: &State) -> Option<RewriteState<'_>> {
    let page = state.range_diff.as_ref()?;
    Some(RewriteState {
        target: &page.target,
        title: &page.title,
        read: match &page.read {
            PairsRead::Comparing => RangeDiffRead::Comparing,
            PairsRead::Read(read) => RangeDiffRead::Read(read),
            PairsRead::Failed(said) => RangeDiffRead::Failed(said),
        },
        opened: &page.opened,
        lit: page.lit,
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
