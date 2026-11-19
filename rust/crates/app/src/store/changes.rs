//! The Changes page and the commit box (`changes.pane`, `commit.create`, `commit.amend`): the message typed, kept for
//! each repository between runs, **Amend last commit**, the folds and the layout of the list; the message is given to
//! the commit when it asks for it, and emptied once the commit is made.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use model::changes::{ChangesInput, ChangesList, FileStatus, UnsavedInput, ignore_candidates, list, unsaved};
use model::contract::{Answer, FeatureId, FollowUp, Question, Target};
use model::details::{FilePath, TreeNode, file_tree};
use model::state::{ChangeKind, FileChange};
use views::changes::{ChangesIntent, ChangesRepository, ChangesState, CommitBox, CommitKind};
use views::details::FilesIntent;
use views::feature::FeatureIntent;
use views::page::Page;
use views::repo::NoRepositoryState;

use super::feature::{Ended, Reply};
use super::{Effect, JobId, JobKind, Shown, State, feature, pages};
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
    /// The message being drafted by the AI provider (`commit.draft-message`).
    pub drafting: Option<super::draft::Drafting>,
    /// The files with edits not saved git was asked about, in the read of the repository they were asked against, and
    /// those of them it ignores (`changes.unsaved`).
    pub checked: Option<Checked>,
    /// The question to git running, and what it asks about.
    pub checking: Option<(JobId, Checked)>,
    /// The file shown beside the list, while one is (`changes.beside-file`).
    pub beside: Option<super::changes_beside::Beside>,
    /// The options of the next commit of each repository changed from where they start (`commit.options`).
    pub options: BTreeMap<PathBuf, model::commit::CommitOptions>,
    /// The picker of the co-authors, while it is open (`commit.co-authors`).
    pub co_authors: Option<super::co_authors::Picking>,
    /// Whether the group of the files hidden from git is folded, while the window is open (`changes.skipped-list`).
    pub is_hidden_folded: bool,
}

/// The files with edits not saved that git was asked whether it ignores.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Checked {
    /// The top folder of the working tree.
    pub top: PathBuf,
    /// The read of the repository.
    pub generation: u64,
    /// The files asked about.
    pub asked: Vec<String>,
    /// Those git ignores.
    pub ignored: Vec<String>,
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
            super::changes_beside::leave(state);
            pages::show_page(state, Page::Changes);
            state.changes.wants_focus = true;
            Vec::new()
        }
        ChangesIntent::Focused => {
            state.changes.wants_focus = false;
            Vec::new()
        }
        ChangesIntent::DraftMessage | ChangesIntent::StopDraft => super::draft::reduce(state, &intent),
        ChangesIntent::OpenBeside(file) => super::changes_beside::open(state, file),
        ChangesIntent::StepFile(step) => super::changes_beside::step(state, step),
        ChangesIntent::Options(asked) => super::commit_options::reduce(state, asked),
        ChangesIntent::CoAuthors(asked) => super::co_authors::reduce(state, asked),
        ChangesIntent::ShowWhole => {
            super::changes_beside::leave(state);
            pages::show_page(state, Page::Changes);
            Vec::new()
        }
        ChangesIntent::FoldHidden => {
            state.changes.is_hidden_folded = !state.changes.is_hidden_folded;
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
    let answers = super::commit_options::answers(state);
    feature::reduce(state, FeatureIntent::RunWith(FeatureId(feature), Target::Head, answers))
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

/// An operation ended: once a commit is made, the message box is emptied, **Amend last commit** unticked and the
/// options of the commit are back where they start; the message git prepared for the next commit fills the box when
/// it is empty.
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
            super::commit_options::made(state, &folder);
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
        draft: if state.changes.drafting.is_some() {
            views::changes::Draft::Writing
        } else {
            views::changes::Draft::Idle
        },
    }
}

/// What the Changes page and the **Changes** section of the side column are drawn from, made for each frame while
/// either is there.
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
    /// The list of the repository shown, while the Changes page is open or the side column, whose **Changes** section
    /// lists it and counts it in its heading (`changes.section`), is not put away.
    #[must_use]
    pub fn new(state: &State) -> ChangesData {
        let is_in_side = !super::timeline::workspace_panes(state).is_rail_hidden || state.is_side_column_over;
        if !is_in_side && !state.pages.open.contains(&Page::Changes) {
            return ChangesData::default();
        }
        let Some(list) = listed(state) else {
            return ChangesData::default();
        };
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
        ChangesState::Repository(Box::new(ChangesRepository {
            list: &self.list,
            files: &self.files,
            tree: &self.tree,
            layout: state.persisted.file_layouts.changes,
            folded: &state.changes.folded,
            commit: commit_box(state),
            identity: &read.state.identity,
            options: Box::new(super::commit_options::options_state(state, &read.folder, &read.state)),
            is_running: state.is_operation_running,
            wants_focus: state.changes.wants_focus,
            on_click: state.settings.open_on_click,
            banner: super::conflicts::banner_state(state, read).map(Box::new),
            worktree: super::worktrees::banner_state(state, read),
            marked: state.changes.beside.as_ref().and_then(|beside| beside.path.as_deref()),
            serial: state.changes.beside.as_ref().map_or(0, |beside| beside.serial),
            skipped: &read.state.skipped,
            is_hidden_folded: state.changes.is_hidden_folded,
            resolved_as_before: &read.state.resolved_as_before,
        }))
    }
}

/// The one list of the changed files of the repository shown, as the Changes page lists them (`changes.list`); none
/// while no repository is read.
#[must_use]
pub fn listed(state: &State) -> Option<ChangesList> {
    let read = state.shown_read()?;
    let unsaved = unsaved_files(state);
    Some(list(&ChangesInput {
        working_tree: &read.state.working_tree,
        unsaved: &unsaved,
    }))
}

/// What the files with edits not saved are worked out from, for the repository shown: the editors' files, its
/// submodules, the other worktrees inside it, and what git said it ignores when it was asked about them.
fn unsaved_input<'a>(
    state: &'a State,
    (buffers, submodules, worktrees): (&'a [String], &'a [String], &'a [String]),
) -> UnsavedInput<'a> {
    let checked = state.changes.checked.as_ref().filter(|checked| {
        state
            .shown_read()
            .is_some_and(|read| crate::persisted::same_folder(&checked.top, &read.state.folder))
    });
    UnsavedInput {
        buffers,
        submodules,
        worktrees,
        checked: checked.map_or((&[], &[]), |checked| {
            (checked.asked.as_slice(), checked.ignored.as_slice())
        }),
    }
}

/// The parts of the repository shown the files with edits not saved are worked out from: the editors' files, its
/// submodules and the other worktrees inside it, each from its top.
fn unsaved_parts(state: &State) -> (Vec<String>, Vec<String>, Vec<String>) {
    let Some(read) = state.shown_read() else {
        return (Vec::new(), Vec::new(), Vec::new());
    };
    let top = &read.state.folder;
    let buffers = super::file_page::unsaved_paths(state, top);
    let submodules = read
        .state
        .submodules
        .iter()
        .map(|submodule| submodule.folder.clone())
        .collect();
    let worktrees = read
        .state
        .worktrees
        .iter()
        .filter(|worktree| !worktree.is_current)
        .filter_map(|worktree| worktree.folder.strip_prefix(top).ok())
        .map(|inside| inside.to_string_lossy().replace('\\', "/"))
        .collect();
    (buffers, submodules, worktrees)
}

/// The files of the repository shown with edits not saved, as the Changes page lists them (`changes.unsaved`).
#[must_use]
pub fn unsaved_files(state: &State) -> Vec<String> {
    let (buffers, submodules, worktrees) = unsaved_parts(state);
    unsaved(&unsaved_input(state, (&buffers, &submodules, &worktrees)))
}

/// Asks git which of the files with edits not saved it ignores, when they are not those last asked about in the read
/// of the repository shown: a file becomes or stops being unsaved, or the repository is read again; not at every key.
pub fn follow_unsaved(state: &mut State) -> Vec<Effect> {
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let (top, generation) = (read.state.folder.clone(), read.generation);
    let (buffers, submodules, worktrees) = unsaved_parts(state);
    let candidates = ignore_candidates(&unsaved_input(state, (&buffers, &submodules, &worktrees)));
    let wanted = Checked {
        top: top.clone(),
        generation,
        asked: candidates,
        ignored: Vec::new(),
    };
    let is_same = |checked: &Checked| {
        checked.top == wanted.top && checked.generation == wanted.generation && checked.asked == wanted.asked
    };
    let changes = &state.changes;
    if changes.checking.as_ref().is_some_and(|(_, checked)| is_same(checked))
        || (changes.checking.is_none() && changes.checked.as_ref().is_some_and(is_same))
    {
        return Vec::new();
    }
    let mut effects: Vec<Effect> = state
        .changes
        .checking
        .take()
        .map(|(job, _)| {
            let _cancelled = state.jobs.remove(&job);
            Effect::Cancel(job)
        })
        .into_iter()
        .collect();
    if wanted.asked.is_empty() {
        state.changes.checked = Some(wanted);
        return effects;
    }
    let kind = JobKind::ReadIgnored {
        folder: top,
        paths: wanted.asked.clone(),
    };
    let job = super::workspace::start_job(state, kind);
    state.changes.checking = Some((job.id, wanted));
    effects.push(Effect::Start(job));
    effects
}

/// What git said of the files with edits not saved: those it ignores are left out of the Changes page; when it could
/// not say, none is.
pub fn ignored_read(state: &mut State, job: JobId, result: Result<Vec<String>, String>) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    let Some((running, mut checked)) = state.changes.checking.take() else {
        return Vec::new();
    };
    if running != job {
        state.changes.checking = Some((running, checked));
        return Vec::new();
    }
    match result {
        Ok(ignored) => checked.ignored = ignored,
        Err(error) => state.last_error = Some(error),
    }
    state.changes.checked = Some(checked);
    Vec::new()
}
