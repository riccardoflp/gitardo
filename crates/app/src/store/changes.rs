//! The Changes page and the commit box (`changes.pane`, `commit.create`, `commit.amend`): the message typed, kept for
//! each repository between runs, **Amend last commit**, the folds and the layout of the list; the message is given to
//! the commit when it asks for it, and emptied once the commit is made.

use std::collections::BTreeSet;

use model::changes::{ChangesInput, ChangesList, FileStatus, list};
use model::contract::{Answer, FeatureId, FollowUp, Question, Target};
use model::details::{FilePath, TreeNode, file_tree};
use model::state::{ChangeKind, FileChange};
use views::changes::{ChangesIntent, ChangesRepository, ChangesState, CommitBox, CommitKind};
use views::details::FilesIntent;
use views::feature::FeatureIntent;
use views::page::Page;
use views::repo::NoRepositoryState;

use super::feature::{Ended, Reply};
use super::{Effect, Shown, State, feature, pages};
use crate::persisted::changes::{keep_message, message_of};

/// What the Changes page keeps while the window is open.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Changes {
    /// Whether **Amend last commit** is ticked.
    pub is_amend: bool,
    /// The directories of the tree folded, by their paths.
    pub folded: BTreeSet<String>,
    /// Whether the keyboard is to go to the message box when the page is drawn next.
    pub wants_focus: bool,
    /// The message given to the commit running, when it asks for it.
    pub sent: Option<String>,
}

/// What is asked of the Changes page or of the commit box of the draft.
pub fn reduce(state: &mut State, intent: ChangesIntent) -> Vec<Effect> {
    match intent {
        ChangesIntent::Type(text) => {
            if let Some(folder) = state.shown_folder().cloned() {
                keep_message(&mut state.persisted.commit_messages, &folder, &text);
            }
            Vec::new()
        }
        ChangesIntent::Amend(is_amend) => {
            state.changes.is_amend = is_amend && commit_box(state).has_commit;
            Vec::new()
        }
        ChangesIntent::Commit => commit(state),
        ChangesIntent::Files(FilesIntent::Layout(layout)) => {
            state.persisted.file_layouts.changes = layout;
            vec![Effect::WritePersisted(state.persisted.clone())]
        }
        ChangesIntent::Files(FilesIntent::Fold(path)) => {
            if !state.changes.folded.remove(&path) {
                let _folded = state.changes.folded.insert(path);
            }
            Vec::new()
        }
        ChangesIntent::Files(FilesIntent::ShowAll) => Vec::new(),
        ChangesIntent::Open => {
            pages::show_page(state, Page::Changes);
            state.changes.wants_focus = true;
            Vec::new()
        }
        ChangesIntent::Focused => {
            state.changes.wants_focus = false;
            Vec::new()
        }
    }
}

/// **Commit**, **Amend** or `Ctrl+Enter`: runs `commit.create`, or `commit.amend` while **Amend last commit** is
/// ticked, with the message of the box; nothing while the button is disabled.
fn commit(state: &mut State) -> Vec<Effect> {
    let commit = commit_box(state);
    if !commit.can_commit() {
        return Vec::new();
    }
    let feature = if commit.is_amend() {
        "commit.amend"
    } else {
        "commit.create"
    };
    state.changes.sent = Some(commit.message.to_owned());
    feature::reduce(state, FeatureIntent::Run(FeatureId(feature), Target::Head))
}

/// The operation running asks a question: the commit asking for its message gets the one of the box at once; any
/// other question is shown.
pub fn asks(state: &mut State, question: Question) -> Vec<Effect> {
    let is_commit = state.operations.running.as_ref().is_some_and(|running| {
        running.feature == FeatureId("commit.create") || running.feature == FeatureId("commit.amend")
    });
    if is_commit
        && question.key == features::commit::MESSAGE
        && let Some(message) = state.changes.sent.take()
    {
        return vec![Effect::ReplyToFeature(Reply::Answer(Some(Answer::Text(message))))];
    }
    feature::asks(state, question)
}

/// An operation ended: once a commit is made, the message box is emptied and **Amend last commit** unticked; the
/// message git prepared for the next commit fills the box when it is empty.
pub fn ended(state: &mut State, ended: &Ended) {
    state.changes.sent = None;
    let Ended::Ran {
        after,
        prepared_message,
        ..
    } = ended
    else {
        return;
    };
    if let (Some(prepared), Some(folder)) = (prepared_message, state.shown_folder().cloned())
        && message_of(&state.persisted.commit_messages, &folder).trim().is_empty()
    {
        keep_message(&mut state.persisted.commit_messages, &folder, prepared);
    }
    if after.contains(&FollowUp::EmptyCommitMessage) {
        state.changes.is_amend = false;
        if let Some(folder) = state.shown_folder().cloned() {
            keep_message(&mut state.persisted.commit_messages, &folder, "");
        }
    }
}

/// The commit box of the repository shown, which the draft of the timeline shows too.
#[must_use]
pub fn commit_box(state: &State) -> CommitBox<'_> {
    let Some(read) = state.shown_read() else {
        return CommitBox::default();
    };
    CommitBox {
        message: message_of(&state.persisted.commit_messages, &read.folder),
        branch: read.state.checked_out_branch(),
        kind: if state.changes.is_amend {
            CommitKind::Amend
        } else {
            CommitKind::New
        },
        has_commit: read.state.head.commit.is_some(),
        has_changes: !read.state.working_tree.is_clean(),
        is_running: state.is_operation_running,
    }
}

/// What the Changes page is drawn from, made for each frame while it is open.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ChangesData {
    /// The one list of the changed files.
    list: ChangesList,
    /// Each row as a file.
    files: Vec<FileChange>,
    /// The tree of the files.
    tree: Vec<TreeNode>,
}

impl ChangesData {
    /// The list of the repository shown, while the Changes page is open.
    #[must_use]
    pub fn new(state: &State) -> ChangesData {
        let read = match state.shown_read() {
            Some(read) if state.pages.open.contains(&Page::Changes) => read,
            _ => return ChangesData::default(),
        };
        let unsaved = super::file_page::unsaved_paths(state, &read.state.folder);
        let list = list(&ChangesInput {
            working_tree: &read.state.working_tree,
            unsaved: &unsaved,
        });
        let files: Vec<FileChange> = list
            .rows
            .iter()
            .map(|row| FileChange {
                path: row.path.clone(),
                original_path: row.original_path.clone(),
                kind: match row.status {
                    FileStatus::Changed(kind) => kind,
                    FileStatus::Untracked => ChangeKind::Added,
                    FileStatus::Unsaved => ChangeKind::Modified,
                },
            })
            .collect();
        let paths: Vec<FilePath> = files.iter().map(|file| FilePath::new(file.path.clone())).collect();
        ChangesData {
            tree: file_tree(&paths),
            list,
            files,
        }
    }

    /// The Changes page: what the timeline says in its place while no repository is read.
    #[must_use]
    pub fn page_state<'a>(&'a self, state: &'a State) -> ChangesState<'a> {
        let shown = state.workspace.as_ref().map(|workspace| &workspace.shown);
        let read = match shown {
            Some(Shown::Read(read)) => read,
            Some(Shown::NoRepository) => return ChangesState::NoRepository(NoRepositoryState::NoRepository),
            Some(Shown::Looking) if state.reads.is_not_read => {
                return ChangesState::NoRepository(NoRepositoryState::NotRead);
            }
            Some(Shown::Looking | Shown::Reading { .. }) | None => {
                return ChangesState::NoRepository(NoRepositoryState::Loading);
            }
        };
        ChangesState::Repository(ChangesRepository {
            list: &self.list,
            files: &self.files,
            tree: &self.tree,
            layout: state.persisted.file_layouts.changes,
            folded: &state.changes.folded,
            commit: commit_box(state),
            identity: &read.state.identity,
            is_running: state.is_operation_running,
            wants_focus: state.changes.wants_focus,
            on_click: state.settings.open_on_click,
            banner: super::conflicts::banner_state(state, read),
            worktree: super::worktrees::banner_state(state, read),
        })
    }
}
