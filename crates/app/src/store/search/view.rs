//! What the views of the search are drawn from: the search box, the bar of a search shown as a list, the rows found,
//! **Go to commit** and the entries of Quick search, made from the store for each frame.

use model::graph::Row;
use model::search::SearchMode;
use views::graph::Selected;
use views::search::{
    GoToCommitDialog, GoToCommitVerdict, HistorySearch, ListMode, MatchCount, Narrowed, QuickChoice, QuickCommand,
    QuickEntry, QuickSearchState, SearchBox,
};
use views::text::search as words;

use super::super::state::State;
use super::Found;

/// The most loaded commits Quick search lists, the newest first.
const MOST_QUICK_COMMITS: usize = 2_000;

/// The entries of Quick search, made only while it is open.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SearchView {
    /// The entries, in order: the commands, the branches, the remote branches, the tags, the commits loaded.
    quick: Vec<QuickEntry>,
}

impl SearchView {
    /// The entries of a state.
    #[must_use]
    pub fn new(state: &State) -> SearchView {
        if !state.search.quick.is_open {
            return SearchView::default();
        }
        SearchView {
            quick: quick_entries(state),
        }
    }

    /// Quick search in the header.
    #[must_use]
    pub fn quick<'a>(&'a self, state: &'a State) -> QuickSearchState<'a> {
        QuickSearchState {
            typed: &state.search.quick.typed,
            is_open: state.search.quick.is_open,
            entries: &self.quick,
        }
    }
}

/// The search box at the top of the timeline.
#[must_use]
pub fn search_box(state: &State) -> SearchBox<'_> {
    let search = &state.search;
    let is_listing = state.timeline.is_reading_again
        && search
            .list
            .as_ref()
            .is_some_and(|asked| matches!(asked.mode, SearchMode::Changes | SearchMode::ChangesPattern));
    let count = match &search.found {
        Found::Nothing => None,
        Found::Commits { commits, has_more } => Some(MatchCount {
            selected: match &state.timeline.selected {
                Some(Selected::Commit(selected)) => {
                    commits.iter().position(|commit| commit == selected).map(|at| at + 1)
                }
                _ => None,
            },
            total: commits.len(),
            has_more: *has_more,
        }),
    };
    SearchBox {
        mode: search.mode,
        text: &search.text,
        found: HistorySearch {
            is_looking: search.matches_job.is_some() || search.hash_job.is_some(),
            is_listing,
            count,
        },
        is_hash_offered: search.is_hash_offered && search.failure.is_none(),
        funnel: search.mode.history().map(|_| state.persisted.search_as_list),
        message: search.failure.as_deref(),
    }
}

/// The bar of a search shown as a list, of the history of a commit alone, or of a file, some lines or a folder, with the branches shown a search is inside
/// and the commits listed; it takes the place of the bar of the branches shown.
#[must_use]
pub fn list_mode(state: &State) -> Option<ListMode<'_>> {
    let search = &state.search;
    let narrowed = match (&search.list, &search.alone, &search.history) {
        (Some(asked), _, _) => Narrowed::Search {
            mode: asked.mode,
            text: &asked.text,
        },
        (None, Some(commit), _) => Narrowed::Commit(commit.short()),
        (None, None, Some(history)) => Narrowed::History {
            history,
            head: state.shown_read().and_then(|read| read.state.head.commit.as_ref()),
        },
        (None, None, None) => return None,
    };
    let timeline = &state.timeline;
    Some(ListMode {
        narrowed,
        within: timeline.shown.as_ref(),
        listed: timeline.read_rows().iter().filter(|row| row.id().is_some()).count(),
        has_more: timeline.has_more,
    })
}

/// Whether the rows a search did not find are dimmed: while it found something to show, in the timeline and not as a
/// list, and git did not refuse it.
#[must_use]
pub fn dims_others(state: &State) -> bool {
    let search = &state.search;
    !search.text.trim().is_empty()
        && search.failure.is_none()
        && search.list.is_none()
        && matches!(search.found, Found::Commits { .. })
}

/// **Go to commit**, while it is open.
#[must_use]
pub fn go_to_commit(state: &State) -> Option<GoToCommitDialog<'_>> {
    let dialog = state.search.dialog.as_ref()?;
    Some(GoToCommitDialog {
        text: &dialog.text,
        verdict: match &dialog.verdict {
            None => GoToCommitVerdict::Unknown,
            Some(Some(_)) => GoToCommitVerdict::Commit,
            Some(None) => GoToCommitVerdict::NoCommit,
        },
        purpose: dialog.purpose,
    })
}

/// Whether a command of Quick search runs: one of the window, or one that runs an operation that is built.
#[must_use]
pub fn is_built(command: QuickCommand) -> bool {
    command.is_built()
        || command
            .operation()
            .is_some_and(|feature| features::operation(feature).is_some())
}

/// The entries of Quick search: the commands, then the branches, the remote branches and the tags of the repository
/// shown, then its commits loaded.
fn quick_entries(state: &State) -> Vec<QuickEntry> {
    let mut entries: Vec<QuickEntry> = QuickCommand::ALL
        .into_iter()
        .map(|command| QuickEntry {
            label: words::command_name(command).to_owned(),
            detail: if is_built(command) {
                words::COMMAND
            } else {
                words::NOT_YET
            }
            .to_owned(),
            choice: QuickChoice::Command(command),
        })
        .collect();
    let Some(read) = state.shown_read() else {
        return entries;
    };
    let repository = &read.state;
    let reference = |name: &str, detail: &str, commit: &model::CommitId| QuickEntry {
        label: name.to_owned(),
        detail: detail.to_owned(),
        choice: QuickChoice::Commit(commit.clone()),
    };
    entries.extend(
        repository
            .branches
            .iter()
            .map(|branch| reference(&branch.name, words::BRANCH, &branch.commit)),
    );
    entries.extend(
        repository
            .remote_branches
            .iter()
            .map(|branch| reference(&branch.name, words::REMOTE_BRANCH, &branch.commit)),
    );
    entries.extend(
        repository
            .tags
            .iter()
            .map(|tag| reference(&tag.name, words::TAG, &tag.commit)),
    );
    let commits = state.timeline.rows.iter().filter_map(|row| match row {
        Row::Commit(commit) => Some(commit),
        Row::Stash { .. } | Row::Uncommitted { .. } => None,
    });
    entries.extend(commits.take(MOST_QUICK_COMMITS).map(|commit| QuickEntry {
        label: format!("{} {}", commit.id.short(), commit.subject),
        detail: words::commit_detail(&commit.author_name),
        choice: QuickChoice::Commit(commit.id.clone()),
    }));
    entries
}
