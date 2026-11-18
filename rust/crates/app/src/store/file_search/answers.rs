//! What the reads of the Search page say, taken into the page: the files listed, then the names ranked and the lines
//! of text as they are found. What a read stopped says is dropped.

use std::sync::Arc;

use super::{FileSearchPage, list_rows};
use crate::store::message::{Effect, JobId};
use crate::store::state::State;
use crate::work::file_search::{Names, SearchAnswer};

/// Takes what a read of the Search page says.
pub fn answered(state: &mut State, job: JobId, answer: SearchAnswer) -> Vec<Effect> {
    if answer.is_last() {
        let _finished = state.jobs.remove(&job);
    }
    let page = &mut state.file_search;
    match answer {
        SearchAnswer::Names(result) => listed(page, job, result),
        answer => found(page, job, answer),
    }
    Vec::new()
}

/// Takes the files listed. When git could not list them, what it said shows in place of the results, until the
/// repository is read again or what narrows the files changes.
fn listed(page: &mut FileSearchPage, job: JobId, result: Result<Arc<Names>, String>) {
    let Some((_, request, generation)) = page.listing.take_if(|(listing, ..)| *listing == job) else {
        return;
    };
    match result {
        Ok(names) => {
            page.failed = None;
            page.names = Some((names, generation));
        }
        Err(error) => {
            let none = Names {
                request,
                ..Names::default()
            };
            page.failed = Some(error);
            page.names = Some((Arc::new(none), generation));
        }
    }
}

/// Takes what the search running says: the names ranked first, then the lines of text as they are found, shown at
/// once, or kept apart until the search ends when the results shown are those of the same search before the
/// repository was read again.
fn found(page: &mut FileSearchPage, job: JobId, answer: SearchAnswer) {
    let Some(finding) = page.finding.as_mut().filter(|finding| finding.job == job) else {
        return;
    };
    let mode = finding.wanted.search.mode;
    match (answer, &mut finding.kept_apart) {
        (SearchAnswer::Names(_), _) => {}
        (SearchAnswer::Ranked(ranked), Some(apart)) => *apart = *ranked,
        (SearchAnswer::Text(lines), Some(apart)) => apart.text.extend(lines),
        (SearchAnswer::Ranked(ranked), None) => {
            page.found = *ranked;
            page.found_for = None;
            page.refused = None;
            list_rows(page, mode, false);
        }
        (SearchAnswer::Text(lines), None) => {
            page.found.text.extend(lines);
            list_rows(page, mode, true);
        }
        (SearchAnswer::Done { has_more, refused }, _) => {
            let Some(finding) = page.finding.take() else {
                return;
            };
            if let Some(apart) = finding.kept_apart {
                page.found = apart;
            }
            page.found.more_text = has_more;
            page.refused = refused;
            page.found_for = Some(finding.wanted);
            list_rows(page, mode, true);
        }
    }
}
