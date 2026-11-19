//! What is done to the files of the changes (`changes.row-menu`, `changes.pane`): the actions of one changed file and
//! of one file in conflict, which the menu of its row lists, and the actions on every file, which the heading of the
//! list has as buttons; each with its words, its icon and what it asks.

use model::changes::{ChangeRow, FileStatus, OpenAction, OpenOnClick, StagedBox};
use model::contract::{Answer, AnswerKey, Answers, FeatureId, Target};
use model::state::{ChangeKind, ConflictedFile};
use ui::Icon;

use super::{BesideFile, ChangesIntent, ChangesRepository};
use crate::details::{FileLayout, FilesIntent};
use crate::feature::FeatureIntent;
use crate::file_view::FileViewIntent;
use crate::icons;
use crate::intent::Intent;
use crate::text::changes as words;

/// Something done to one file of the changes, or to all of them: an item of a menu, or a button.
#[derive(Clone, Debug, PartialEq)]
pub struct RowAction {
    /// Its words.
    pub words: &'static str,
    /// Its icon, the same on a button and in a menu.
    pub icon: Icon,
    /// What it asks; none for an action whose operation comes later, which cannot be chosen.
    pub intent: Option<Intent>,
    /// Whether it changes the repository or a file, and so cannot be chosen while an operation runs.
    pub is_change: bool,
    /// Whether it is the first of a group, which a menu puts under a line.
    pub is_group_start: bool,
}

impl RowAction {
    /// An action that shows something and changes nothing.
    pub(super) fn shows(words: &'static str, icon: Icon, intent: Option<Intent>) -> RowAction {
        RowAction {
            words,
            icon,
            intent,
            is_change: false,
            is_group_start: false,
        }
    }

    /// An action that changes the repository or a file.
    fn changes(words: &'static str, icon: Icon, intent: Intent) -> RowAction {
        RowAction {
            words,
            icon,
            intent: Some(intent),
            is_change: true,
            is_group_start: false,
        }
    }

    /// The same action, the first of its group.
    fn starting_group(self) -> RowAction {
        RowAction {
            is_group_start: true,
            ..self
        }
    }

    /// Whether it can be chosen: it asks something, and no operation runs when it changes something.
    #[must_use]
    pub fn is_offered(&self, is_running: bool) -> bool {
        self.intent.is_some() && !(self.is_change && is_running)
    }
}

/// The intent that runs an operation of the catalog on what it is invoked on.
pub(super) fn run(feature: &'static str, target: Target) -> Intent {
    Intent::Feature(FeatureIntent::Run(FeatureId(feature), target))
}

/// What opens, beside the list, the page `changes.open` decides on, when it is the Diff page or the File page
/// (`changes.beside-file`): a file in conflict is not in this list.
pub(super) fn open_page(row: &ChangeRow, action: OpenAction, on_click: OpenOnClick) -> Option<Intent> {
    let file = super::beside_request(row, action, on_click)?;
    Some(Intent::Changes(ChangesIntent::OpenBeside(file)))
}

/// What can be done to a changed file, in the order its menu lists them: **Open changes** and **Open file**, the
/// second only while the file is still on disk; then **Stage**, or **Unstage** on a file wholly staged, **Discard
/// changes** and **Stash this file…**; then **Ignore…** and, on a file git tracks, **Stop tracking, keep on disk…**, on
/// a file still on disk; then **Save** on a file with edits not saved. A file only an editor has changed
/// has **Save** and **Compare with the saved file**, whose operation comes later.
#[must_use]
pub fn file_actions(row: &ChangeRow) -> Vec<RowAction> {
    let save = Intent::FileView(FileViewIntent::Save(row.path.clone()));
    if row.status == FileStatus::Unsaved {
        return vec![
            RowAction::changes(words::SAVE, icons::SAVE, save),
            RowAction::shows(words::COMPARE_WITH_SAVED, icons::CHANGES, None),
        ];
    }
    let file = || Target::File(row.path.clone());
    let open_changes = open_page(row, OpenAction::OpenChanges, OpenOnClick::File);
    let mut actions = vec![RowAction::shows(words::OPEN_CHANGES, icons::CHANGES, open_changes)];
    if row.status != FileStatus::Changed(ChangeKind::Deleted) {
        let open_file = Intent::Changes(ChangesIntent::OpenBeside(BesideFile::File(row.path.clone())));
        actions.push(RowAction::shows(words::OPEN_FILE, icons::OPEN_FILE, Some(open_file)));
    }
    let staging = if row.staged == StagedBox::Ticked {
        RowAction::changes(words::UNSTAGE, icons::UNSTAGE, run("changes.unstage", file()))
    } else {
        RowAction::changes(words::STAGE, icons::ADD, run("changes.stage", file()))
    };
    actions.push(staging.starting_group());
    actions.push(RowAction::changes(
        words::DISCARD,
        icons::UNDO,
        run("changes.discard", file()),
    ));
    let stash = run("stash.push-file", Target::Files(vec![row.path.clone()]));
    actions.push(RowAction::changes(words::STASH_FILE, icons::STASH, stash));
    if row.status != FileStatus::Changed(ChangeKind::Deleted) {
        let ignore = run("changes.ignore", file());
        actions.push(RowAction::changes(words::IGNORE, icons::HIDE, ignore).starting_group());
        if row.status != FileStatus::Untracked {
            let stop = run("changes.stop-tracking", file());
            actions.push(RowAction::changes(words::STOP_TRACKING, icons::UNSTAGE, stop));
        }
    }
    if row.is_unsaved {
        actions.push(RowAction::changes(words::SAVE, icons::SAVE, save).starting_group());
    }
    actions
}

/// What can be done to a file in conflict, in the order its menu lists them: the sparkle's, with `conflicts.ai` on,
/// which opens the review of its conflicts and asks the AI about them (`ai.resolve-conflicts`); the wand's, with
/// `conflicts.autoResolve` on, which opens the review of its conflicts (`review.page`); then **Mark as resolved**
/// (`conflicts.mark-resolved`) and the two sides (`conflicts.take-side`).
#[must_use]
pub fn conflict_actions(state: &ChangesRepository<'_>, file: &ConflictedFile) -> Vec<RowAction> {
    let take = |side: &str| {
        let mut answers = Answers::new();
        answers.insert(AnswerKey("side"), Answer::Choice(side.to_owned()));
        let feature = FeatureId("conflicts.take-side");
        Intent::Feature(FeatureIntent::RunWith(
            feature,
            Target::File(file.path.clone()),
            answers,
        ))
    };
    let only = || Some(vec![file.path.clone()]);
    let mut actions = Vec::new();
    if state.banner.is_some_and(|banner| banner.ai) {
        let review = Intent::Review(crate::review::ReviewIntent::ResolveWithAi(only()));
        actions.push(RowAction::changes(words::RESOLVE_WITH_AI, icons::DRAFTED, review));
    }
    if state.banner.is_some_and(|banner| banner.auto_resolve) {
        let review = Intent::Review(crate::review::ReviewIntent::Open(only()));
        actions.push(RowAction::shows(
            words::RESOLVE_AUTOMATICALLY,
            icons::BY_THE_RULES,
            Some(review),
        ));
    }
    let mark = run("conflicts.mark-resolved", Target::File(file.path.clone()));
    let mark = RowAction::changes(words::MARK_AS_RESOLVED, icons::ADD, mark);
    actions.push(if actions.is_empty() {
        mark
    } else {
        mark.starting_group()
    });
    actions.push(RowAction::changes(
        words::TAKE_INCOMING,
        icons::TAKE_INCOMING,
        take("incoming"),
    ));
    actions.push(RowAction::changes(
        words::TAKE_CURRENT,
        icons::TAKE_CURRENT,
        take("current"),
    ));
    actions
}

/// What is done to every file of the list, as the heading of the list has them from its right: **Discard all
/// changes**, **Save all** while a file has edits not saved, **Stash all changes…**, **Stash only the staged changes…**
/// while something is staged, **Open all changes in one Diff page**, and **View as tree** or **View as list**.
#[must_use]
pub fn heading_actions(state: &ChangesRepository<'_>) -> Vec<RowAction> {
    let mut actions = vec![RowAction::changes(
        words::DISCARD_ALL,
        icons::UNDO,
        run("changes.discard-all", Target::Changes),
    )];
    if state.list.has_unsaved() {
        let save_all = Intent::FileView(FileViewIntent::SaveAll);
        actions.push(RowAction::changes(words::SAVE_ALL, icons::SAVE, save_all));
    }
    let stash_all = run("stash.push", Target::Head);
    actions.push(RowAction::changes(words::STASH_ALL, icons::STASH, stash_all));
    if state.list.has_staged() {
        let stash_staged = run("stash.push-staged", Target::Head);
        actions.push(RowAction::changes(words::STASH_STAGED, icons::STASH, stash_staged));
    }
    let open_all = crate::diff::changes_files(&state.list.rows)
        .map(|target| Intent::Changes(ChangesIntent::OpenBeside(BesideFile::Diff(Box::new(target)))));
    actions.push(RowAction::shows(
        words::OPEN_ALL_CHANGES,
        icons::OPEN_ALL_CHANGES,
        open_all,
    ));
    let (layout, icon, said) = match state.layout {
        FileLayout::List => (FileLayout::Tree, icons::TREE, words::VIEW_AS_TREE),
        FileLayout::Tree => (FileLayout::List, icons::LIST, words::VIEW_AS_LIST),
    };
    let other = Intent::Changes(ChangesIntent::Files(FilesIntent::Layout(layout)));
    actions.push(RowAction::shows(said, icon, Some(other)));
    actions
}

/// How many of the buttons of the heading stay on it, of a count, in a width that holds a number of them: all when
/// they fit; otherwise one place less than fit, the last being **…**, which has the others.
#[must_use]
pub fn buttons_kept(count: usize, places: usize) -> usize {
    if count <= places {
        count
    } else {
        places.saturating_sub(1)
    }
}

/// The order the buttons of the heading leave it for **…** when the width does not hold them, the first to go first
/// (Q762): the layout, **Stash only the staged changes…**, **Save all**, **Stash all changes…**, **Discard all
/// changes**, and **Open all changes in one Diff page** last.
const LEAVING: [&str; 7] = [
    words::VIEW_AS_TREE,
    words::VIEW_AS_LIST,
    words::STASH_STAGED,
    words::SAVE_ALL,
    words::STASH_ALL,
    words::DISCARD_ALL,
    words::OPEN_ALL_CHANGES,
];

/// The actions of the heading that stay on it as buttons when only some do, by their places in the list: those that
/// leave last.
#[must_use]
pub fn staying(actions: &[RowAction], kept: usize) -> Vec<usize> {
    let rank = |action: &RowAction| LEAVING.iter().position(|words| *words == action.words);
    let mut order: Vec<usize> = (0..actions.len()).collect();
    order.sort_by_key(|index| std::cmp::Reverse(actions.get(*index).and_then(rank)));
    order.truncate(kept);
    order.sort_unstable();
    order
}
