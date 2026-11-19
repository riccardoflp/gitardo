//! The inspector as the store keeps it (`details.show`): which row its details are of, their read on a worker, and what
//! the user changed in it: **Show more** (`details.show-more`), **Show all** (`details.show-all-files`) and the folds of
//! the tree (`details.file-tree`), all of which start again when another row is selected.

use std::collections::BTreeSet;
use std::path::PathBuf;

use git::read::details::CommitRequest;
use model::CommitId;
use model::details::{CommitDetail, FilePath, TreeNode, file_tree};
use model::state::FileChange;
use text::details::{RawMessage, message_title};
use views::details::{DetailsIntent, DetailsRead, DetailsState, FILE_CAP, FilesIntent, ReadDetails, SelectedDetails};
use views::graph::Selected;

use super::message::{Effect, JobId, JobKind};
use super::state::State;

/// What the details of the inspector are of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    /// The folder of the repository.
    pub folder: PathBuf,
    /// The row.
    pub row: SubjectRow,
}

/// The row the details of the inspector are of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubjectRow {
    /// A commit or a stash.
    Commit(CommitId),
    /// The draft, as a read of the repository found it: a new read reads its files again.
    Draft(u64),
}

/// How far the details of the inspector are read.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum InspectorRead {
    /// They are read.
    #[default]
    Loading,
    /// They are read, the message shaped for the inspector.
    Read(Box<ReadDetails>),
    /// The commit is not in the repository any more.
    Missing,
    /// Git could not read them.
    Failed,
}

/// The inspector.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Inspector {
    /// What its details are of; none while nothing is selected.
    pub subject: Option<Subject>,
    /// The read of the details, while it runs.
    pub job: Option<JobId>,
    /// The details read.
    pub read: InspectorRead,
    /// Whether **Show more** was clicked.
    pub is_message_expanded: bool,
    /// Whether **Show all** was clicked.
    pub is_all_files_shown: bool,
    /// The directories of the tree folded, by their paths.
    pub folded: BTreeSet<String>,
    /// The tree of the files shown.
    pub tree: Vec<TreeNode>,
}

/// Follows the row selected: when another one is, the read of the last one is cancelled, the details of the new one
/// are read, and **Show more**, **Show all** and the folds start again. The draft read again keeps what it showed until
/// its new files come.
pub fn follow(state: &mut State) -> Vec<Effect> {
    let wanted = wanted(state);
    if wanted == state.inspector.subject {
        return Vec::new();
    }
    let mut effects = Vec::new();
    if let Some(job) = state.inspector.job.take() {
        let _cancelled = state.jobs.remove(&job);
        effects.push(Effect::Cancel(job));
    }
    let is_same_draft = matches!(
        (&wanted, &state.inspector.subject),
        (Some(new), Some(old)) if new.folder == old.folder
            && matches!((&new.row, &old.row), (SubjectRow::Draft(_), SubjectRow::Draft(_)))
    );
    if !is_same_draft {
        state.inspector = Inspector::default();
    }
    if let Some(subject) = wanted {
        let request = match &subject.row {
            SubjectRow::Commit(commit) => CommitRequest::Commit(commit.clone()),
            SubjectRow::Draft(_) => CommitRequest::UncommittedChanges,
        };
        let kind = JobKind::ReadDetails {
            folder: subject.folder.clone(),
            request,
        };
        let job = super::workspace::start_job(state, kind);
        state.inspector.job = Some(job.id);
        state.inspector.subject = Some(subject);
        effects.push(Effect::Start(job));
    }
    effects
}

/// What the details are to be of: the row selected in the timeline of the repository shown.
fn wanted(state: &State) -> Option<Subject> {
    let read = state.shown_read()?;
    let row = match state.timeline.selected.as_ref()? {
        Selected::Commit(commit) => SubjectRow::Commit(commit.clone()),
        Selected::Draft => SubjectRow::Draft(read.generation),
    };
    Some(Subject {
        folder: read.folder.clone(),
        row,
    })
}

/// Takes the details read, when they are those of the row selected still.
pub fn details_read(state: &mut State, job: JobId, result: Result<Option<Box<CommitDetail>>, String>) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    if state.inspector.job != Some(job) {
        return Vec::new();
    }
    let inspector = &mut state.inspector;
    inspector.job = None;
    inspector.read = match result {
        Ok(Some(detail)) => InspectorRead::Read(Box::new(shape(*detail))),
        Ok(None) => InspectorRead::Missing,
        Err(error) => {
            state.last_error = Some(error);
            InspectorRead::Failed
        }
    };
    let inspector = &mut state.inspector;
    if let InspectorRead::Read(read) = &inspector.read {
        inspector.tree = tree_of(&read.detail.files, inspector.is_all_files_shown);
    }
    Vec::new()
}

/// The details of a commit with its message shaped as the inspector and the page show it (`details.message-title`,
/// `details.message-unwrap`).
#[must_use]
pub fn shape(detail: CommitDetail) -> ReadDetails {
    let shown = message_title(&RawMessage { text: &detail.message });
    ReadDetails {
        detail,
        title: shown.title,
        text: shown.text,
    }
}

/// The tree of the files shown: the first 300, or all of them once **Show all** was clicked.
#[must_use]
pub fn tree_of(files: &[FileChange], is_all_shown: bool) -> Vec<TreeNode> {
    let shown = if is_all_shown {
        files.len()
    } else {
        files.len().min(FILE_CAP)
    };
    let paths: Vec<FilePath> = files
        .iter()
        .take(shown)
        .map(|file| FilePath::new(file.path.clone()))
        .collect();
    file_tree(&paths)
}

/// Folds a directory of a tree, or unfolds it.
pub fn toggle_fold(folded: &mut BTreeSet<String>, path: String) {
    if !folded.remove(&path) {
        let _new = folded.insert(path);
    }
}

/// Changes the inspector as the user asked.
pub fn reduce_details(state: &mut State, intent: DetailsIntent) -> Vec<Effect> {
    match intent {
        DetailsIntent::Close => {
            state.timeline.selected = None;
            state.timeline.several.clear();
            super::timeline::retrace(state);
            Vec::new()
        }
        DetailsIntent::OpenPage(commit) => match state.shown_read().map(|read| read.folder.clone()) {
            Some(folder) => super::commit_page::open(state, folder, commit),
            None => Vec::new(),
        },
        DetailsIntent::ShowMore(is_expanded) => {
            state.inspector.is_message_expanded = is_expanded;
            Vec::new()
        }
        DetailsIntent::Files(FilesIntent::ShowAll) => {
            let inspector = &mut state.inspector;
            inspector.is_all_files_shown = true;
            if let InspectorRead::Read(read) = &inspector.read {
                inspector.tree = tree_of(&read.detail.files, true);
            }
            Vec::new()
        }
        DetailsIntent::Files(FilesIntent::Layout(layout)) => {
            state.persisted.file_layouts.inspector = layout;
            vec![Effect::WritePersisted(state.persisted.clone())]
        }
        DetailsIntent::Files(FilesIntent::Fold(path)) => {
            toggle_fold(&mut state.inspector.folded, path);
            Vec::new()
        }
    }
}

/// What the inspector is drawn from, for the row selected as the timeline read it.
#[must_use]
pub fn details_state<'a>(state: &'a State, selected: Option<SelectedDetails<'a>>) -> DetailsState<'a> {
    let inspector = &state.inspector;
    DetailsState {
        selected,
        read: match &inspector.read {
            InspectorRead::Loading => DetailsRead::Loading,
            InspectorRead::Read(read) => DetailsRead::Read(read),
            InspectorRead::Missing => DetailsRead::Missing,
            InspectorRead::Failed => DetailsRead::Failed,
        },
        layout: state.persisted.file_layouts.inspector,
        is_message_expanded: inspector.is_message_expanded,
        is_all_files_shown: inspector.is_all_files_shown,
        tree: &inspector.tree,
        folded: &inspector.folded,
        in_place: super::compare::in_place::in_place_state(state),
        shown_file: super::diff_page::beside::shown_file(state),
        shallow_ends: state
            .shown_read()
            .map_or(&[], |read| read.state.shallow.ends.as_slice()),
    }
}
