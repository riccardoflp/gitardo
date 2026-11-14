//! Keeping the blame of the File page in front up to date: the line of its cursor read 200 ms after the cursor rests
//! (`blame.current-line`), and its file read when it is asked for or every file shows its blame, again 400 ms after the
//! last edit and at once when the commit checked out or `blame.ignoreWhitespaceAndMoves` changes (`blame.file`). A read
//! no longer wanted is cancelled, and the blame of the pages closed is forgotten.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::Duration;

use views::file_view::FilePageId;
use views::page::Page;

use super::{Inputs, PageBlame, Wanted};
use crate::persisted::same_folder;
use crate::store::message::{Effect, JobId, JobKind};
use crate::store::state::State;
use crate::work::blame::BlameJob;

/// How long the cursor rests on a line before its blame is read.
const LINE_REST: Duration = Duration::from_millis(200);
/// How long after the last edit the blame of a file is read again.
const EDIT_REST: Duration = Duration::from_millis(400);

/// Forgets the blame of the pages closed, and starts the reads the page in front wants.
pub fn follow(state: &mut State) -> Vec<Effect> {
    let mut effects = prune(state);
    if let Some(Page::File(id)) = state.page_shown() {
        effects.extend(follow_file(state, id));
        effects.extend(follow_line(state, id));
    }
    effects
}

/// Forgets the blame of the pages closed, cancelling their reads.
fn prune(state: &mut State) -> Vec<Effect> {
    let open: Vec<FilePageId> = state.file_pages.pages.iter().map(|page| page.id).collect();
    let closed: Vec<FilePageId> = state
        .blames
        .pages
        .keys()
        .filter(|id| !open.contains(id))
        .copied()
        .collect();
    let mut jobs = Vec::new();
    for id in closed {
        if let Some(page) = state.blames.pages.remove(&id) {
            jobs.extend(page.file_job);
            jobs.extend(page.line_job);
        }
    }
    if state.blames.hovered.is_some_and(|(id, _)| !open.contains(&id)) {
        state.blames.hovered = None;
    }
    cancel(state, jobs)
}

/// Cancels some reads.
fn cancel(state: &mut State, jobs: Vec<JobId>) -> Vec<Effect> {
    jobs.into_iter()
        .map(|job| {
            let _cancelled = state.jobs.remove(&job);
            Effect::Cancel(job)
        })
        .collect()
}

/// Cancels the read of the blame of the file of a page.
pub(super) fn cancel_file(state: &mut State, id: FilePageId) -> Vec<Effect> {
    let job = state.blames.pages.get_mut(&id).and_then(|page| page.file_job.take());
    cancel(state, job.into_iter().collect())
}

/// What a read of the blame of a page would be read with now; none while its file is not read as text in the
/// repository shown.
pub(super) fn inputs_of(state: &State, id: FilePageId) -> Option<Inputs> {
    let read = state.shown_read()?;
    let page = state.file_pages.pages.iter().find(|page| page.id == id)?;
    if !same_folder(&page.top, &read.state.folder) {
        return None;
    }
    let document = page.document()?;
    Some(Inputs {
        head: read.state.head.commit.clone(),
        ignores_whitespace: state.settings.features.blame.ignores_whitespace,
        text: fingerprint(document.buffer().text()),
    })
}

/// A fingerprint of a text.
fn fingerprint(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// Reads the blame of the file of the page in front when it shows it, or every file does, and what it was read with
/// changed; takes it away when it showed only because every file did and that was turned off.
fn follow_file(state: &mut State, id: FilePageId) -> Vec<Effect> {
    let Some(inputs) = inputs_of(state, id) else {
        return Vec::new();
    };
    let Some(file) = state.file_pages.pages.iter().find(|page| page.id == id) else {
        return Vec::new();
    };
    let is_on_disk = file.revision.is_none();
    let is_on_every_file = state.settings.features.blame.is_on_every_file;
    let page = state.blames.pages.entry(id).or_default();
    if page.wanted == Wanted::EveryFile && !is_on_every_file {
        *page = PageBlame {
            is_hidden_until_closed: page.is_hidden_until_closed,
            ..PageBlame::default()
        };
        return cancel_file(state, id);
    }
    let has_none = page.has_none_at.as_ref() == Some(&inputs.head);
    if page.wanted == Wanted::No && is_on_every_file && is_on_disk && !page.is_hidden_until_closed && !has_none {
        page.wanted = Wanted::EveryFile;
    }
    if page.wanted == Wanted::No || page.file_asked.as_ref() == Some(&inputs) || (has_none && page.lines.is_none()) {
        return Vec::new();
    }
    let only_typed = page
        .file_asked
        .as_ref()
        .is_some_and(|asked| asked.head == inputs.head && asked.ignores_whitespace == inputs.ignores_whitespace);
    page.file_asked = Some(inputs);
    let delay = if only_typed { EDIT_REST } else { Duration::ZERO };
    let mut effects = cancel_file(state, id);
    effects.extend(start(state, id, (None, delay)));
    effects
}

/// Reads the blame of the line of the cursor of the page in front once it rests on another line, or the text changed,
/// while `blame.enabled` is on, the page shows a file on disk and the selection is within one line; forgets it
/// otherwise.
fn follow_line(state: &mut State, id: FilePageId) -> Vec<Effect> {
    let is_enabled = state.settings.features.blame.is_enabled;
    let asked = state
        .file_pages
        .pages
        .iter()
        .find(|page| page.id == id)
        .and_then(|page| {
            let buffer = page.document()?.buffer();
            let range = page.cursor.range();
            let (start, end) = (buffer.position(range.start).line, buffer.position(range.end).line);
            let is_wanted = is_enabled && page.revision.is_none() && start == end;
            is_wanted.then(|| (start, fingerprint(buffer.text())))
        });
    let asked = asked.filter(|_| inputs_of(state, id).is_some());
    let page = state.blames.pages.entry(id).or_default();
    if page.line_asked == asked {
        return Vec::new();
    }
    if asked.map(|(line, _)| line) != page.line_asked.map(|(line, _)| line) {
        page.line = None;
    }
    page.line_asked = asked;
    let job = page.line_job.take();
    let mut effects = cancel(state, job.into_iter().collect());
    if let Some((line, _)) = asked {
        effects.extend(start(state, id, (Some(line + 1), LINE_REST)));
    }
    effects
}

/// Starts a read of the blame of a page: of a line, from 1, or of the whole file, after a while.
fn start(state: &mut State, id: FilePageId, (line, delay): (Option<usize>, Duration)) -> Vec<Effect> {
    let Some(page) = state.file_pages.pages.iter().find(|page| page.id == id) else {
        return Vec::new();
    };
    let job = BlameJob {
        path: page.path.clone(),
        revision: page.revision.clone(),
        unsaved: page
            .is_unsaved()
            .then(|| page.document().map(|document| document.buffer().to_bytes()))
            .flatten(),
        ignores_whitespace: state.settings.features.blame.ignores_whitespace,
        line,
        delay,
    };
    let kind = JobKind::ReadBlame {
        folder: page.top.clone(),
        job,
    };
    let job = crate::store::workspace::start_job(state, kind);
    if let Some(page) = state.blames.pages.get_mut(&id) {
        if line.is_some() {
            page.line_job = Some(job.id);
        } else {
            page.file_job = Some(job.id);
        }
    }
    vec![Effect::Start(job)]
}
