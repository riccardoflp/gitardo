//! The words of the search: the search box at the top of the timeline and what it finds (`search.box`,
//! `search.history`, `search.list-mode`, `search.go-to-hash`), **Go to commit…** (`search.go-to-commit`) and Quick search
//! in the header.

use model::CommitId;
use model::file_history::{HistoryOf, LineRange};
use model::filter::BranchesShown;
use model::search::{BoxMode, SearchMode};

use super::filter;
use crate::search::Narrowed;

/// The button that steps to the previous match.
pub const PREVIOUS_MATCH: &str = "Previous match (Shift+Enter, Shift+F3)";
/// The button that steps to the next match.
pub const NEXT_MATCH: &str = "Next match (Enter, F3)";
/// The tooltip of a count with a `+`.
pub const FIRST_MATCHES_COUNTED: &str = "Only the first matches were counted";
/// The funnel at the end of the box.
pub const FUNNEL: &str = "Show only the commits found, as a list, instead of highlighting them in the timeline";
/// What stands in the place of the count while a hash typed is not among the loaded commits.
pub const ENTER: &str = "Enter \u{21b5}";
/// Its tooltip.
pub const ENTER_TOOLTIP: &str = "Press Enter to go to the commit with this hash, wherever it is in the history";
/// What the box says while the list of a search in the changes is read.
pub const SEARCHING: &str = "Searching\u{2026}";
/// What a search git refused says, before the first line of git's message.
pub const SEARCH_FAILED: &str = "Gitardo: the search failed.";
/// The button of the bar of a search shown as a list that shows the whole timeline again.
pub const SHOW_ALL_COMMITS: &str = "Show all commits";
/// The bar of a search shown as a list, for the screen reader.
pub const LIST_BAR: &str = "Only the commits found";

/// What asks for a search of the whole history, in the Activity tab.
pub const SEARCHING_HISTORY: &str = "Searching the history";
/// What asks for the commit of a name or a hash, in the Activity tab.
pub const LOOKING_FOR_COMMIT: &str = "Looking for a commit";
/// An entry of Quick search that opens the settings page at a section or a setting: `Open Settings: Worktrees`.
#[must_use]
pub fn open_settings_at(name: &str) -> String {
    format!("Open Settings: {name}")
}

/// What an entry of Quick search that opens the settings page at a section is.
pub const SETTINGS_SECTION: &str = "Section of the settings";
/// What an entry of Quick search that opens the settings page at a setting is.
pub const SETTING: &str = "Setting";

/// The title of a read of the activity of a search: `Searching app`.
#[must_use]
pub fn searching(name: &str) -> String {
    format!("Searching {name}")
}

/// The name of a mode in the list before the box.
#[must_use]
pub fn mode_name(mode: BoxMode) -> &'static str {
    match mode {
        BoxMode::Loaded => "Loaded",
        BoxMode::Message => "Message",
        BoxMode::Author => "Author",
        BoxMode::Changes => "Changes",
        BoxMode::ChangesPattern => "Changes (regex)",
    }
}

/// What the empty box says in a mode.
#[must_use]
pub fn placeholder(mode: BoxMode) -> &'static str {
    match mode {
        BoxMode::Loaded => "Find by message, author or hash",
        BoxMode::Message => "Text in the message, then Enter",
        BoxMode::Author => "Author name or email, then Enter",
        BoxMode::Changes => "Text added or removed, then Enter",
        BoxMode::ChangesPattern => "Regex on changed lines, then Enter",
    }
}

/// What a mode runs, in its tooltip.
#[must_use]
pub fn mode_tooltip(mode: BoxMode) -> &'static str {
    match mode {
        BoxMode::Loaded => {
            "Highlight matches among the loaded commits. Enter on a hash goes to that commit, however far back it is"
        }
        BoxMode::Message => "git log --grep: search every commit message",
        BoxMode::Author => "git log --author: search every commit author",
        BoxMode::Changes => "git log -S: commits that add or remove this text",
        BoxMode::ChangesPattern => {
            "git log -G: commits whose changed lines match this POSIX extended regular expression ([0-9], not \\d)"
        }
    }
}

/// The count of the matches: `14`, `2/14`, `14+` or `2/14+`.
#[must_use]
pub fn count(selected: Option<usize>, total: usize, has_more: bool) -> String {
    let more = if has_more { "+" } else { "" };
    match selected {
        Some(place) => format!("{place}/{total}{more}"),
        None => format!("{total}{more}"),
    }
}

/// What a search git refused says: [`SEARCH_FAILED`] and the first line of git's message.
#[must_use]
pub fn search_failed(message: &str) -> String {
    let first = message
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    if first.is_empty() {
        SEARCH_FAILED.to_owned()
    } else {
        format!("{SEARCH_FAILED} {first}")
    }
}

/// What a hash typed that is no commit says (`search.go-to-hash`).
#[must_use]
pub fn not_a_commit(typed: &str) -> String {
    format!("Gitardo: \"{typed}\" is not a commit of this repository, or is the start of several.")
}

/// The bar above the commits of a search shown as a list or of the history of a commit alone, in the words of the bar
/// of `filter.bar`: `Commits by authors matching alice · in the local branches · 2 commits`, `Only the history of
/// 4b830a6 · on no branch or tag · 3 commits`.
#[must_use]
pub fn list_bar(narrowed: &Narrowed<'_>, within: Option<&BranchesShown>, counted: (usize, bool)) -> (String, String) {
    let what = match narrowed {
        Narrowed::History { history, head } => {
            let (what, mut dimmed) = history_bar(history, *head);
            // The history of some lines is followed from one commit, so the branches shown do not narrow it.
            if let (Some(shown), false) = (within, matches!(history, HistoryOf::Lines { .. })) {
                dimmed.insert(0, format!("in {}", filter::narrowed_to(shown)));
            }
            dimmed.push(filter::commits_words(counted));
            return (what, format!("\u{b7} {}", dimmed.join(" \u{b7} ")));
        }
        Narrowed::Search { mode, text } => {
            let words = match mode {
                SearchMode::Message => format!("Commits whose message contains {text}"),
                SearchMode::Author => format!("Commits by authors matching {text}"),
                SearchMode::Changes => format!("Commits that add or remove {text}"),
                SearchMode::ChangesPattern => format!("Commits whose changed lines match {text}"),
            };
            match within {
                Some(shown) => format!("{words} \u{b7} in {}", filter::narrowed_to(shown)),
                None => words,
            }
        }
        Narrowed::Commit(short) => format!("Only the history of {short} \u{b7} on no branch or tag"),
    };
    (
        format!("{what} \u{b7} {}", filter::commits_words(counted)),
        String::new(),
    )
}

/// What the bar of the history of a file, of some lines or of a folder says, then what it adds, dimmed: `History of
/// src/b.rs`, then `earlier src/a.rs`; `History of lines 4–9 of src/app.rs`, then `lines 5–10 in the editor` and `as
/// committed in 4b830a6` once another commit is checked out.
#[must_use]
pub fn history_bar(history: &HistoryOf, head: Option<&CommitId>) -> (String, Vec<String>) {
    match history {
        HistoryOf::File { path, earlier } => {
            let dimmed = if earlier.is_empty() {
                Vec::new()
            } else {
                vec![format!("earlier {}", earlier.join(", "))]
            };
            (format!("History of {path}"), dimmed)
        }
        HistoryOf::Lines {
            path,
            lines,
            commit,
            in_editor,
        } => {
            let mut dimmed = Vec::new();
            if let Some(in_editor) = in_editor.filter(|in_editor| in_editor != lines) {
                dimmed.push(format!("{} in the editor", lines_words(in_editor)));
            }
            if head != Some(commit) {
                dimmed.push(format!("as committed in {}", commit.short()));
            }
            (format!("History of {} of {path}", lines_words(*lines)), dimmed)
        }
        HistoryOf::Folder { path } => (format!("History of the folder {path}"), Vec::new()),
    }
}

/// Some lines, in words: `line 4`, `lines 4–9`.
#[must_use]
pub fn lines_words(lines: LineRange) -> String {
    if lines.start == lines.end {
        format!("line {}", lines.start)
    } else {
        format!("lines {}\u{2013}{}", lines.start, lines.end)
    }
}

/// The title of **Go to commit**.
pub const GO_TO_COMMIT_TITLE: &str = "Go to commit";
/// What **Go to commit** asks for.
pub const GO_TO_COMMIT_DETAIL: &str =
    "A commit hash, or anything else git understands: HEAD~3, v1.0^, main@{yesterday}";
/// What **Go to commit** says under its box while git knows no commit by what is typed.
pub const UNKNOWN_COMMIT: &str = "Git does not know a commit by this name in this repository.";
/// The button of **Go to commit** that goes.
pub const GO: &str = "Go to commit";
/// The button of **Go to commit** that closes it.
pub const CANCEL: &str = "Cancel";
/// The title of **Open commit**, which **Open Commit as a Page…** of Quick search opens (`commit-page.open`).
pub const OPEN_COMMIT_TITLE: &str = "Open commit";
/// The button of **Open commit** that opens the page of the commit.
pub const OPEN_PAGE: &str = "Open as a page";
/// **Go to commit…** in the **…** menu of the header.
pub const GO_TO_COMMIT_ENTRY: &str = "Go to commit\u{2026}";

/// What the picker of Quick search says while nothing is typed in it.
pub const QUICK_SEARCH: &str = "Commits, branches, files, commands";
/// What a command of Quick search says beside it while what it runs is not built yet.
pub const NOT_YET: &str = "Not yet";
/// What a branch of Quick search says beside it.
pub const BRANCH: &str = "Branch";
/// What a remote branch of Quick search says beside it.
pub const REMOTE_BRANCH: &str = "Remote branch";
/// What a tag of Quick search says beside it.
pub const TAG: &str = "Tag";
/// What a command of Quick search says beside it.
pub const COMMAND: &str = "Command";

/// What a commit of Quick search says beside it: its author.
#[must_use]
pub fn commit_detail(author: &str) -> String {
    format!("Commit \u{b7} {author}")
}

/// The name of a command of Quick search, as its card says it.
#[must_use]
pub fn command_name(command: crate::search::QuickCommand) -> &'static str {
    use crate::search::QuickCommand;
    match command {
        QuickCommand::OpenTimeline => "Open Timeline",
        QuickCommand::GoToCommit => "Go to Commit\u{2026}",
        QuickCommand::OpenCommitAsPage => "Open Commit as a Page\u{2026}",
        QuickCommand::Refresh => "Refresh",
        QuickCommand::OpenFolder => "Open folder\u{2026}",
        QuickCommand::CloneRepository => "Clone Repository\u{2026}",
        QuickCommand::CreateRepository => "Create Repository\u{2026}",
        QuickCommand::SelectRepository => "Select Repository",
        QuickCommand::OpenWorkspaceFile => super::repo::OPEN_WORKSPACE_FILE,
        QuickCommand::OpenRepositoryInTab => "Open Repository in a Tab",
        QuickCommand::CloseTab => "Close Tab",
        QuickCommand::ShowWhatIsRunning => "Show What Is Running",
        QuickCommand::ShowBranches => "Show branches",
        QuickCommand::FilterBranchRail => "Filter the Branch Rail",
        QuickCommand::Welcome => "Welcome",
        QuickCommand::ShowChanges => "Show changes",
        QuickCommand::CreateBranch => "Create branch\u{2026}",
        QuickCommand::Fetch => "Fetch",
        QuickCommand::Push => "Push",
        QuickCommand::StashChanges => "Stash Changes",
        QuickCommand::ShowStashes => "Show Stashes",
        QuickCommand::Compare => "Compare Branches, Tags or Commits\u{2026}",
        QuickCommand::BrowseReflog => "Browse reflog",
        QuickCommand::CleanUpBranches => "Clean up branches",
        QuickCommand::ManageRemotes => "Manage remotes",
        QuickCommand::SwitchWorktree => "Switch Worktree\u{2026}",
        QuickCommand::CreateWorktree => "Create Worktree\u{2026}",
        QuickCommand::AddSubmodule => "Add Submodule\u{2026}",
        QuickCommand::ApplyPatch => "Apply Patch\u{2026}",
        QuickCommand::SwitchIdentity => "Switch identity",
        QuickCommand::SignInToGitHub => super::github::SIGN_IN,
        QuickCommand::ShowCommandLog => "Show Command Log",
        QuickCommand::ShowTerminal => "Show Terminal",
        QuickCommand::NewTerminal => "New Terminal",
        QuickCommand::RunTask => super::tasks_tab::RUN_TASK,
        QuickCommand::ShowTasks => super::tasks_tab::SHOW_TASKS,
        QuickCommand::ImportGroups => super::tasks_tab::IMPORT_GROUPS,
        QuickCommand::ExportGroups => super::tasks_tab::EXPORT_GROUPS,
        QuickCommand::UndoLastOperation => "Undo Last Operation",
        QuickCommand::ReviewConflicts => crate::text::review::COMMAND,
        QuickCommand::OpenSettings => "Open Settings",
        QuickCommand::ShowFiles => crate::text::files::COMMAND,
        QuickCommand::SearchInFiles => crate::text::file_search::COMMAND,
        QuickCommand::ChooseAiModel => "Choose the model that resolves conflicts",
        QuickCommand::TrustedFolders => "Trusted folders\u{2026}",
        QuickCommand::ChooseWhatNewWorktreesGet => crate::text::worktree_setup::COMMAND,
        QuickCommand::ShowFileHistory => crate::text::blame::SHOW_FILE_HISTORY,
        QuickCommand::ShowLinesHistory => crate::text::blame::SHOW_LINES_HISTORY,
        QuickCommand::ToggleFileBlame => crate::text::blame::TOGGLE_FILE_BLAME,
        QuickCommand::ToggleInlineBlame => crate::text::blame::TOGGLE_INLINE_BLAME,
        QuickCommand::OpenLineCommit => crate::text::blame::OPEN_LINE_COMMIT,
        QuickCommand::RevealLineCommit => crate::text::blame::REVEAL_LINE_COMMIT,
        QuickCommand::BlamePrevious => crate::text::blame::BLAME_BEFORE,
        QuickCommand::ToggleIgnoreWhitespace => crate::text::blame::TOGGLE_IGNORE_WHITESPACE,
        QuickCommand::ToggleEveryFile => crate::text::blame::TOGGLE_EVERY_FILE,
    }
}
