//! The icon of each meaning, as the table of `graph.icons` gives it. A view names the meaning of what it draws, never
//! an icon: an icon is used for one meaning only, so that the same action looks the same everywhere.

use ui::Icon;

#[cfg(test)]
mod tests;

/// Folded: a row whose content is hidden; a menu that opens to the side.
pub(crate) const FOLDED: Icon = Icon::ChevronRight;

/// Unfolded.
pub(crate) const UNFOLDED: Icon = Icon::ChevronDown;

/// The next change or match, in the bar of a page.
pub(crate) const NEXT: Icon = Icon::ChevronDown;

/// The previous change or match.
pub(crate) const PREVIOUS: Icon = Icon::ChevronUp;

/// A button that opens a list of choices below it.
pub(crate) const CHOOSE: Icon = Icon::ChevronsUpDown;

/// Close a page, a drawer, the inspector, a box, the bottom panel; empty a box.
pub(crate) const CLOSE: Icon = Icon::Close;

/// Chosen in a menu; done; a good commit of a bisect.
pub(crate) const CHOSEN: Icon = Icon::Check;

/// Accept every change that needs no choice.
pub(crate) const ACCEPT_SIMPLE: Icon = Icon::CheckAll;

/// Add: stage a file or a change, take a side into the result, a new terminal, a submodule, a tab.
pub(crate) const ADD: Icon = Icon::Plus;

/// Unstage.
pub(crate) const UNSTAGE: Icon = Icon::Minus;

/// Back to how it was: discard the changes of a file, revert a change, Undo.
pub(crate) const UNDO: Icon = Icon::Undo;

/// Delete: a branch, a tag, a stash dropped, a terminal killed, an entry of a list.
pub(crate) const DELETE: Icon = Icon::Delete;

/// Save, save all.
pub(crate) const SAVE: Icon = Icon::Save;

/// Search.
pub(crate) const SEARCH: Icon = Icon::Search;

/// Filter what is shown or listed.
pub(crate) const FILTER: Icon = Icon::Filter;

/// Show, show beside.
pub(crate) const SHOW: Icon = Icon::Eye;

/// Hide.
#[allow(dead_code, reason = "a meaning of the table that no button of the window draws yet")]
pub(crate) const HIDE: Icon = Icon::EyeOff;

/// Branch.
pub(crate) const BRANCH: Icon = Icon::Branch;

/// Create a branch.
pub(crate) const CREATE_BRANCH: Icon = Icon::CreateBranch;

/// Tag.
pub(crate) const TAG: Icon = Icon::Tag;

/// Remote.
pub(crate) const REMOTE: Icon = Icon::Cloud;

/// Commit.
pub(crate) const COMMIT: Icon = Icon::Commit;

/// Open a commit as a page.
pub(crate) const OPEN_AS_PAGE: Icon = Icon::OpenAsPage;

/// Merge.
#[allow(dead_code, reason = "a meaning of the table that no button of the window draws yet")]
pub(crate) const MERGE: Icon = Icon::Merge;

/// Cherry-pick.
#[allow(dead_code, reason = "a meaning of the table that no button of the window draws yet")]
pub(crate) const CHERRY_PICK: Icon = Icon::CherryPick;

/// Compare; the Compare view.
pub(crate) const COMPARE: Icon = Icon::Compare;

/// Swap the two sides.
pub(crate) const SWAP: Icon = Icon::Swap;

/// Pull request.
pub(crate) const PULL_REQUEST: Icon = Icon::PullRequest;

/// A draft pull request.
pub(crate) const DRAFT_PULL_REQUEST: Icon = Icon::DraftPullRequest;

/// Fetch.
#[allow(dead_code, reason = "a meaning of the table that no button of the window draws yet")]
pub(crate) const FETCH: Icon = Icon::Fetch;

/// Pull.
#[allow(dead_code, reason = "a meaning of the table that no button of the window draws yet")]
pub(crate) const PULL: Icon = Icon::Pull;

/// Push.
#[allow(dead_code, reason = "a meaning of the table that no button of the window draws yet")]
pub(crate) const PUSH: Icon = Icon::Push;

/// Sync.
pub(crate) const SYNC: Icon = Icon::Sync;

/// Commits to pull, beside their count.
pub(crate) const TO_PULL: Icon = Icon::ArrowDown;

/// Commits to push, beside their count.
pub(crate) const TO_PUSH: Icon = Icon::ArrowUp;

/// Read again.
pub(crate) const READ_AGAIN: Icon = Icon::Refresh;

/// Back to the top.
pub(crate) const BACK_TO_TOP: Icon = Icon::ToTop;

/// Move up in a list.
pub(crate) const MOVE_UP: Icon = Icon::MoveUp;

/// Move down in a list.
pub(crate) const MOVE_DOWN: Icon = Icon::MoveDown;

/// The timeline: the Timeline view, show in the timeline.
pub(crate) const TIMELINE: Icon = Icon::Timeline;

/// Changes: the Changes view; open the changes of a file.
pub(crate) const CHANGES: Icon = Icon::FileDiff;

/// The Files view.
pub(crate) const FILES: Icon = Icon::FolderTree;

/// The Terminal view.
pub(crate) const TERMINAL: Icon = Icon::Terminal;

/// What is running; the Activity view.
pub(crate) const RUNNING: Icon = Icon::Activity;

/// Settings.
pub(crate) const SETTINGS: Icon = Icon::Gear;

/// The Command log.
#[allow(dead_code, reason = "a meaning of the table that no button of the window draws yet")]
pub(crate) const COMMAND_LOG: Icon = Icon::CommandLog;

/// Folder.
pub(crate) const FOLDER: Icon = Icon::Folder;

/// File.
pub(crate) const FILE: Icon = Icon::File;

/// Open the file.
pub(crate) const OPEN_FILE: Icon = Icon::FileText;

/// Open all the changes.
pub(crate) const OPEN_ALL_CHANGES: Icon = Icon::Files;

/// Worktree.
pub(crate) const WORKTREE: Icon = Icon::Worktree;

/// Submodule.
pub(crate) const SUBMODULE: Icon = Icon::Submodule;

/// Stash.
pub(crate) const STASH: Icon = Icon::Stash;

/// Apply a stash.
pub(crate) const APPLY: Icon = Icon::Apply;

/// Pop a stash.
pub(crate) const POP: Icon = Icon::Pop;

/// Favorite.
pub(crate) const FAVORITE: Icon = Icon::Star;

/// Recent.
pub(crate) const RECENT: Icon = Icon::Clock;

/// History of a file or a folder.
pub(crate) const HISTORY: Icon = Icon::History;

/// Who changed each line.
pub(crate) const BLAME: Icon = Icon::Blame;

/// Who changed each line, on every file.
pub(crate) const ON_EVERY_FILE: Icon = Icon::Pin;

/// Drafted by the AI; proposed for a conflict.
pub(crate) const DRAFTED: Icon = Icon::Sparkles;

/// Resolved by the rules.
pub(crate) const BY_THE_RULES: Icon = Icon::Wand;

/// Edited, not saved or not committed.
pub(crate) const EDITED: Icon = Icon::Pencil;

/// Waits.
pub(crate) const WAITS: Icon = Icon::Hourglass;

/// Did not run, skipped.
pub(crate) const SKIPPED: Icon = Icon::Skipped;

/// Warning.
pub(crate) const WARNING: Icon = Icon::Warning;

/// Error; a bad commit of a bisect.
pub(crate) const ERROR: Icon = Icon::Error;

/// Information.
pub(crate) const INFORMATION: Icon = Icon::Info;

/// More.
pub(crate) const MORE: Icon = Icon::More;

/// List.
pub(crate) const LIST: Icon = Icon::List;

/// Tree of folders.
pub(crate) const TREE: Icon = Icon::ListTree;

/// Fold the lines that did not change.
pub(crate) const FOLD_UNCHANGED: Icon = Icon::FoldUnchanged;

/// Stop what is running: drafting a message.
pub(crate) const STOP: Icon = Icon::Stop;

/// Open in the external editor.
pub(crate) const OPEN_EXTERNAL: Icon = Icon::External;

/// Take the incoming version of a file, or the version of the other side of a comparison.
pub(crate) const TAKE_INCOMING: Icon = Icon::TakeIncoming;

/// Take the current version of a file.
pub(crate) const TAKE_CURRENT: Icon = Icon::TakeCurrent;

/// Drag to move a row.
pub(crate) const DRAG: Icon = Icon::Grip;

/// A bisect.
pub(crate) const BISECT: Icon = Icon::Bisect;

/// A submodule not initialized.
pub(crate) const NOT_INITIALIZED: Icon = Icon::NotInitialized;

/// The base of a merge, shown or hidden.
pub(crate) const BASE: Icon = Icon::Base;

/// The look of the window: the section **Appearance** of the settings.
pub(crate) const APPEARANCE: Icon = Icon::Palette;

/// Each meaning of the table of `graph.icons` and its icon, in the order of the table.
#[cfg(test)]
pub(crate) const MEANINGS: [(&str, Icon); 81] = [
    (
        "Folded: a row whose content is hidden; a menu that opens to the side",
        FOLDED,
    ),
    ("Unfolded", UNFOLDED),
    ("The next change or match, in the bar of a page", NEXT),
    ("The previous change or match", PREVIOUS),
    ("A button that opens a list of choices below it", CHOOSE),
    (
        "Close a page, a drawer, the inspector, a box, the bottom panel; empty a box",
        CLOSE,
    ),
    ("Chosen in a menu; done; a good commit of a bisect", CHOSEN),
    ("Accept every change that needs no choice", ACCEPT_SIMPLE),
    (
        "Add: stage a file or a change, take a side into the result, a new terminal, a submodule, a tab",
        ADD,
    ),
    ("Unstage", UNSTAGE),
    (
        "Back to how it was: discard the changes of a file, revert a change, Undo",
        UNDO,
    ),
    (
        "Delete: a branch, a tag, a stash dropped, a terminal killed, an entry of a list",
        DELETE,
    ),
    ("Save, save all", SAVE),
    ("Search", SEARCH),
    ("Filter what is shown or listed", FILTER),
    ("Show, show beside", SHOW),
    ("Hide", HIDE),
    ("Branch", BRANCH),
    ("Create a branch", CREATE_BRANCH),
    ("Tag", TAG),
    ("Remote", REMOTE),
    ("Commit", COMMIT),
    ("Open a commit as a page", OPEN_AS_PAGE),
    ("Merge", MERGE),
    ("Cherry-pick", CHERRY_PICK),
    ("Compare; the Compare view", COMPARE),
    ("Swap the two sides", SWAP),
    ("Pull request", PULL_REQUEST),
    ("A draft pull request", DRAFT_PULL_REQUEST),
    ("Fetch", FETCH),
    ("Pull", PULL),
    ("Push", PUSH),
    ("Sync", SYNC),
    ("Commits to pull, beside their count", TO_PULL),
    ("Commits to push, beside their count", TO_PUSH),
    ("Read again", READ_AGAIN),
    ("Back to the top", BACK_TO_TOP),
    ("Move up in a list", MOVE_UP),
    ("Move down in a list", MOVE_DOWN),
    ("The timeline: the Timeline view, show in the timeline", TIMELINE),
    ("Changes: the Changes view; open the changes of a file", CHANGES),
    ("The Files view", FILES),
    ("The Terminal view", TERMINAL),
    ("What is running; the Activity view", RUNNING),
    ("Settings", SETTINGS),
    ("The Command log", COMMAND_LOG),
    ("Folder", FOLDER),
    ("File", FILE),
    ("Open the file", OPEN_FILE),
    ("Open all the changes", OPEN_ALL_CHANGES),
    ("Worktree", WORKTREE),
    ("Submodule", SUBMODULE),
    ("Stash", STASH),
    ("Apply a stash", APPLY),
    ("Pop a stash", POP),
    ("Favorite", FAVORITE),
    ("Recent", RECENT),
    ("History of a file or a folder", HISTORY),
    ("Who changed each line", BLAME),
    ("Who changed each line, on every file", ON_EVERY_FILE),
    ("Drafted by the AI; proposed for a conflict", DRAFTED),
    ("Resolved by the rules", BY_THE_RULES),
    ("Edited, not saved or not committed", EDITED),
    ("Waits", WAITS),
    ("Did not run, skipped", SKIPPED),
    ("Warning", WARNING),
    ("Error; a bad commit of a bisect", ERROR),
    ("Information", INFORMATION),
    ("More", MORE),
    ("List", LIST),
    ("Tree of folders", TREE),
    ("Fold the lines that did not change", FOLD_UNCHANGED),
    ("Stop what is running: drafting a message", STOP),
    ("Open in the external editor", OPEN_EXTERNAL),
    (
        "Take the incoming version of a file, or the version of the other side of a comparison",
        TAKE_INCOMING,
    ),
    ("Take the current version of a file", TAKE_CURRENT),
    ("Drag to move a row", DRAG),
    ("A bisect", BISECT),
    ("A submodule not initialized", NOT_INITIALIZED),
    ("The base of a merge, shown or hidden", BASE),
    (
        "The look of the window: the section Appearance of the settings",
        APPEARANCE,
    ),
];
