//! What the user changes in a Diff page: the current change chosen with `F7` and `Shift+F7` (`diff.navigate`), on to
//! the next changed file in a diff between `HEAD`, the index and the working tree; the unchanged rows folded or shown
//! (`diff.collapse-unchanged`); and the width of the left side (`diff.resize-sides`).

use std::time::Duration;

use model::diff::{compare_paths, sorted_paths};
use views::diff::{DiffFile, DiffPageId, DiffPageIntent, DiffVersion, FileDiff, Hint};

use super::{DiffPage, lines, page_mut, read_file};
use crate::store::message::Effect;
use crate::store::state::State;
use crate::work::diff::DiffRead;

/// How long a hint of the bar stays.
pub const HINT_TIME: Duration = Duration::from_secs(4);

/// Which way `F7` goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// `F7`: the next change.
    Next,
    /// `Shift+F7`: the previous change.
    Previous,
}

impl Direction {
    /// The hint shown on the last change, or the first, before going on to another file.
    fn hint(self) -> Hint {
        match self {
            Direction::Next => Hint::NextFile,
            Direction::Previous => Hint::PreviousFile,
        }
    }
}

/// Changes a page as the user asked.
pub fn reduce_page(state: &mut State, id: DiffPageId, intent: DiffPageIntent) -> Vec<Effect> {
    match intent {
        DiffPageIntent::Next(at) => return step(state, id, (Direction::Next, at)),
        DiffPageIntent::Previous(at) => return step(state, id, (Direction::Previous, at)),
        DiffPageIntent::Resize(width) => {
            state.diff_pages.last_width = width;
            if let Some(page) = page_mut(state, id) {
                page.left_width = width;
            }
        }
        DiffPageIntent::Collapse(is_collapsed) => {
            if let Some(page) = page_mut(state, id) {
                page.is_collapsed = is_collapsed;
                page.unfolded.iter_mut().for_each(std::collections::BTreeSet::clear);
                lines::rebuild(page);
            }
        }
        DiffPageIntent::Unfold(file, start) => {
            if let Some(page) = page_mut(state, id) {
                if let Some(unfolded) = page.unfolded.get_mut(file) {
                    let _new = unfolded.insert(start);
                }
                lines::rebuild(page);
            }
        }
    }
    Vec::new()
}

/// `F7` or `Shift+F7`: the next or the previous change, centered; from no change, the first or the last. On the last
/// change, or the first going back, a diff between `HEAD`, the index and the working tree says it goes on to the next
/// file, and does when pressed again within 4 seconds; a diff against a commit goes round.
fn step(state: &mut State, id: DiffPageId, (direction, at): (Direction, Duration)) -> Vec<Effect> {
    let serial = state.diff_pages.serial + 1;
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    let total = page.changes.len();
    let next = match (direction, page.current) {
        (_, _) if total == 0 => None,
        (Direction::Next, None) => Some(0),
        (Direction::Previous, None) => Some(total - 1),
        (Direction::Next, Some(current)) => (current + 1 < total).then_some(current + 1),
        (Direction::Previous, Some(current)) => current.checked_sub(1),
    };
    if let Some(next) = next {
        page.hint = None;
        go_to_change(page, next, serial);
        state.diff_pages.serial = serial;
        return Vec::new();
    }
    if !goes_to_files(page) {
        if total > 0 {
            page.hint = None;
            go_to_change(page, if direction == Direction::Next { 0 } else { total - 1 }, serial);
            state.diff_pages.serial = serial;
        }
        return Vec::new();
    }
    let wanted = direction.hint();
    if page.hint.is_some_and(|(hint, until)| hint == wanted && at < until) {
        page.hint = None;
        return go_to_file(state, id, (direction, at));
    }
    page.hint = Some((wanted, at + HINT_TIME));
    Vec::new()
}

/// Makes a change the current one, and brings it to the middle of both sides.
fn go_to_change(page: &mut DiffPage, change: usize, serial: u64) {
    page.current = Some(change);
    if let Some(place) = page.changes.get(change) {
        page.reveal = Some((serial, place.line));
    }
}

/// Whether `F7` goes on to the next changed file: a page of one file between `HEAD`, the index and the working tree.
fn goes_to_files(page: &DiffPage) -> bool {
    page.target.files.len() == 1
        && matches!(
            (&page.target.left, &page.target.right),
            (DiffVersion::Head, DiffVersion::Index)
                | (DiffVersion::Index | DiffVersion::Head, DiffVersion::WorkingTree)
        )
}

/// The files that differ between the two versions of a page, in the order `F7` goes through them: the staged ones for
/// the index against `HEAD`, the unstaged and untracked ones for the working tree against the index, all of them for
/// the working tree against `HEAD`.
fn changed_paths(state: &State, page: &DiffPage) -> Vec<String> {
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let tree = &read.state.working_tree;
    let staged = tree.staged.iter().map(|change| change.path.as_str());
    let unstaged = tree
        .unstaged
        .iter()
        .map(|change| change.path.as_str())
        .chain(tree.untracked.iter().map(String::as_str));
    match (&page.target.left, &page.target.right) {
        (DiffVersion::Head, DiffVersion::Index) => sorted_paths(staged),
        (DiffVersion::Index, DiffVersion::WorkingTree) => sorted_paths(unstaged),
        (DiffVersion::Head, DiffVersion::WorkingTree) => sorted_paths(staged.chain(unstaged)),
        _ => Vec::new(),
    }
}

/// The file after a path among the changed ones, going round, in a direction; none when no other is changed.
fn file_after(paths: &[String], path: &str, direction: Direction) -> Option<String> {
    let others: Vec<&String> = paths.iter().filter(|other| other.as_str() != path).collect();
    let found = match direction {
        Direction::Next => others
            .iter()
            .find(|other| compare_paths(other, path).is_gt())
            .or_else(|| others.first()),
        Direction::Previous => others
            .iter()
            .rev()
            .find(|other| compare_paths(other, path).is_lt())
            .or_else(|| others.last()),
    };
    found.map(|path| (*path).clone())
}

/// Shows in the page the same diff of the next changed file, or says there is no other.
fn go_to_file(state: &mut State, id: DiffPageId, (direction, at): (Direction, Duration)) -> Vec<Effect> {
    let Some(page) = state.diff_pages.pages.iter().find(|page| page.id == id) else {
        return Vec::new();
    };
    let path = page.target.files.first().map_or_default(|file| file.right.clone());
    let from = page
        .going
        .as_ref()
        .map_or_else(|| path.clone(), |(_, from)| from.clone());
    let next = file_after(&changed_paths(state, page), &path, direction);
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    let Some(next) = next else {
        page.going = None;
        page.hint = Some((Hint::NoOtherFiles, at + HINT_TIME));
        return Vec::new();
    };
    page.going = (next != from).then_some((direction, from));
    page.target.files = vec![DiffFile::same(&next)];
    page.title = page.target.page_title();
    page.diffs = vec![FileDiff::Loading];
    page.jobs = vec![None];
    page.unfolded = vec![std::collections::BTreeSet::new()];
    page.current = None;
    page.caret = None;
    lines::rebuild(page);
    read_file(state, id, 0, true).into_iter().collect()
}

/// Takes the diff of a file of an open page: kept as it was when nothing changed, so that nothing is drawn again; a
/// file `F7` went to that cannot be shown is passed over, and the one it lands on starts at its first change, or its
/// last going back.
pub(super) fn shown(state: &mut State, (id, file): (DiffPageId, usize), result: DiffRead) -> Vec<Effect> {
    let diff = match result {
        DiffRead::Text(text) => FileDiff::Text(text),
        DiffRead::Binary => FileDiff::Binary,
        DiffRead::TooComplex | DiffRead::Large => FileDiff::TooComplex,
        DiffRead::Failed(error) => {
            state.last_error = Some(error);
            FileDiff::Failed
        }
    };
    let serial = state.diff_pages.serial + 1;
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    let is_text = matches!(diff, FileDiff::Text(_));
    if let Some((direction, _)) = page.going.clone() {
        if !is_text {
            return go_to_file(state, id, (direction, Duration::ZERO));
        }
        page.going = None;
        if let Some(slot) = page.diffs.get_mut(file) {
            *slot = diff;
        }
        lines::rebuild(page);
        let count = page.changes.len();
        if count > 0 {
            go_to_change(page, if direction == Direction::Next { 0 } else { count - 1 }, serial);
            state.diff_pages.serial = serial;
        }
        return Vec::new();
    }
    if page.diffs.get(file) == Some(&diff) {
        return Vec::new();
    }
    if page.diffs.len() == 1
        && let FileDiff::Text(text) = &diff
        && text.sides == (false, false)
    {
        // The file is on neither side any more, as after `git clean` of an untracked file: the page closes with it.
        super::close(state, id);
        return Vec::new();
    }
    if let Some(slot) = page.diffs.get_mut(file) {
        *slot = diff;
    }
    lines::rebuild(page);
    Vec::new()
}
