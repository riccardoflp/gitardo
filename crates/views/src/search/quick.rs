//! Quick search (`Ctrl+K`, D17): the box in the middle of the header, and the picker it opens of the commands of the
//! catalog that name Quick search, the branches, remote branches and tags, and the commits loaded.

use model::CommitId;
use ui::layout::{Align, place};
use ui::{Icon, InputBox, Key, Modifiers, Picker, PickerAnswer, PickerItem, PickerMode, Ui};

use super::SearchIntent;
use crate::intent::Intent;
use crate::text::header as header_words;
use crate::text::search as words;

/// The widest the box of Quick search is.
const SEARCH_WIDTH: f32 = 420.0;
/// The room the header keeps for the box of Quick search before what is beside it, the activity indicator, takes more.
pub(crate) const SEARCH_LEAST_WIDTH: f32 = 260.0;

/// What Quick search shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct QuickSearchState<'a> {
    /// What was typed in the box of the header, which the picker starts from.
    pub typed: &'a str,
    /// Whether the picker is open.
    pub is_open: bool,
    /// What the picker lists, in order: the commands, the references, the commits.
    pub entries: &'a [QuickEntry],
}

/// An entry of Quick search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuickEntry {
    /// Its words, which what is typed is matched against.
    pub label: String,
    /// What it is, on the right of its row.
    pub detail: String,
    /// What choosing it does.
    pub choice: QuickChoice,
}

/// What choosing an entry of Quick search does.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum QuickChoice {
    /// Runs a command.
    Command(QuickCommand),
    /// Goes to a commit in the timeline: a commit found, or the commit of a reference.
    Commit(CommitId),
}

/// A command of the catalog whose card names Quick search among its entry points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum QuickCommand {
    /// **Open Timeline** (`graph.view`).
    OpenTimeline,
    /// **Go to Commit…** (`search.go-to-commit`).
    GoToCommit,
    /// **Open Commit as a Page…** (`commit-page.open`).
    OpenCommitAsPage,
    /// **Refresh** (`repo.schedule-reads`).
    Refresh,
    /// **Open folder…** (`repo.open-folder`).
    OpenFolder,
    /// **Select Repository** (`repo.select`).
    SelectRepository,
    /// **Show What Is Running** (`activity.what-is-running`).
    ShowWhatIsRunning,
    /// **Show branches** (`refs.tree`).
    ShowBranches,
    /// **Show Command Log** (`log.show`).
    ShowCommandLog,
    /// **Open Settings** (`settings.edit`).
    OpenSettings,
    /// **Welcome** (`welcome.page`).
    Welcome,
    /// **Show changes** (`changes.page`).
    ShowChanges,
    /// **Create branch…** (`branch.create`).
    CreateBranch,
    /// **Fetch** (`sync.fetch`).
    Fetch,
    /// **Push** (`sync.push`).
    Push,
    /// **Stash Changes** (`stash.push`).
    StashChanges,
    /// **Show Stashes** (`stash.pane`).
    ShowStashes,
    /// **Compare Branches, Tags or Commits…** (`compare.choose`).
    Compare,
    /// **Browse reflog** (`reflog.browse`).
    BrowseReflog,
    /// **Clean up branches** (`branch.clean-up`).
    CleanUpBranches,
    /// **Manage remotes** (`remote.manage`).
    ManageRemotes,
    /// **Switch Worktree…** (`worktree.switch`).
    SwitchWorktree,
    /// **Create Worktree…** (`worktree.create`).
    CreateWorktree,
    /// **Add Submodule…** (`submodule.add`).
    AddSubmodule,
    /// **Apply Patch…** (`patch.apply`).
    ApplyPatch,
    /// **Switch identity** (`identity.switch`).
    SwitchIdentity,
    /// **Sign in to GitHub** (`github.load-pull-requests`).
    SignInToGitHub,
    /// **Undo Last Operation** (`undo.last`).
    UndoLastOperation,
    /// **Review the resolutions of the conflicts** (`review.page`).
    ReviewConflicts,
    /// **Show files**: the Files page.
    ShowFiles,
    /// **Choose the model that resolves conflicts** (`ai.choose-model`).
    ChooseAiModel,
    /// **Trusted folders…** (`trust.manage`).
    TrustedFolders,
    /// **Choose What New Worktrees Get…** (`worktree-setup.page`).
    ChooseWhatNewWorktreesGet,
    /// **Show file history** (`file-history.file`), for the File page in front.
    ShowFileHistory,
    /// **Show history of selected lines** (`file-history.lines`).
    ShowLinesHistory,
    /// **Toggle file blame** (`blame.toggle-file`).
    ToggleFileBlame,
    /// **Toggle inline blame** (`blame.toggle-current-line`).
    ToggleInlineBlame,
    /// **Open commit of current line as a page** (`blame.open-line-commit`).
    OpenLineCommit,
    /// **Reveal commit of current line in the timeline** (`blame.reveal-line-commit`).
    RevealLineCommit,
    /// **Blame file before commit of current line** (`blame.previous`).
    BlamePrevious,
    /// **Toggle ignoring whitespace and moved lines in blame** (`blame.ignore-whitespace`).
    ToggleIgnoreWhitespace,
    /// **Toggle file blame in every file** (`blame.every-file`).
    ToggleEveryFile,
}

impl QuickCommand {
    /// Every command, in the order Quick search lists them: those that run now first.
    pub const ALL: [QuickCommand; 42] = [
        QuickCommand::OpenTimeline,
        QuickCommand::GoToCommit,
        QuickCommand::OpenCommitAsPage,
        QuickCommand::Refresh,
        QuickCommand::OpenFolder,
        QuickCommand::SelectRepository,
        QuickCommand::ShowWhatIsRunning,
        QuickCommand::ShowBranches,
        QuickCommand::ShowCommandLog,
        QuickCommand::OpenSettings,
        QuickCommand::Welcome,
        QuickCommand::ShowChanges,
        QuickCommand::ShowFiles,
        QuickCommand::ChooseWhatNewWorktreesGet,
        QuickCommand::ShowFileHistory,
        QuickCommand::ShowLinesHistory,
        QuickCommand::ToggleFileBlame,
        QuickCommand::ToggleInlineBlame,
        QuickCommand::OpenLineCommit,
        QuickCommand::RevealLineCommit,
        QuickCommand::BlamePrevious,
        QuickCommand::ToggleIgnoreWhitespace,
        QuickCommand::ToggleEveryFile,
        QuickCommand::CreateBranch,
        QuickCommand::Fetch,
        QuickCommand::Push,
        QuickCommand::StashChanges,
        QuickCommand::ShowStashes,
        QuickCommand::Compare,
        QuickCommand::BrowseReflog,
        QuickCommand::CleanUpBranches,
        QuickCommand::ManageRemotes,
        QuickCommand::SwitchWorktree,
        QuickCommand::CreateWorktree,
        QuickCommand::AddSubmodule,
        QuickCommand::ApplyPatch,
        QuickCommand::SwitchIdentity,
        QuickCommand::SignInToGitHub,
        QuickCommand::UndoLastOperation,
        QuickCommand::ReviewConflicts,
        QuickCommand::ChooseAiModel,
        QuickCommand::TrustedFolders,
    ];

    /// Whether what the command runs is built: the others are listed with **Not yet**, and choosing them does
    /// nothing.
    #[must_use]
    pub fn is_built(self) -> bool {
        self <= QuickCommand::ShowChanges
            || matches!(
                self,
                QuickCommand::ShowStashes
                    | QuickCommand::ReviewConflicts
                    | QuickCommand::ShowFiles
                    | QuickCommand::ChooseWhatNewWorktreesGet
            )
    }

    /// The operation of the catalog the command runs on the File page in front: on the line of its cursor, or on the
    /// file it shows.
    #[must_use]
    pub fn file_page_operation(self) -> Option<&'static str> {
        match self {
            QuickCommand::ShowFileHistory => Some("file-history.file"),
            QuickCommand::ShowLinesHistory => Some("file-history.lines"),
            QuickCommand::ToggleFileBlame => Some("blame.toggle-file"),
            QuickCommand::ToggleInlineBlame => Some("blame.toggle-current-line"),
            QuickCommand::OpenLineCommit => Some("blame.open-line-commit"),
            QuickCommand::RevealLineCommit => Some("blame.reveal-line-commit"),
            QuickCommand::BlamePrevious => Some("blame.previous"),
            QuickCommand::ToggleIgnoreWhitespace => Some("blame.ignore-whitespace"),
            QuickCommand::ToggleEveryFile => Some("blame.every-file"),
            _ => None,
        }
    }

    /// The operation of the catalog the command runs at `HEAD`, for the commands that run one: the window runs it once
    /// it is built.
    #[must_use]
    pub fn operation(self) -> Option<&'static str> {
        match self {
            QuickCommand::CreateBranch => Some("branch.create"),
            QuickCommand::Fetch => Some("sync.fetch"),
            QuickCommand::Push => Some("sync.push"),
            QuickCommand::StashChanges => Some("stash.push"),
            QuickCommand::CleanUpBranches => Some("branch.clean-up"),
            QuickCommand::ManageRemotes => Some("remote.manage"),
            QuickCommand::AddSubmodule => Some("submodule.add"),
            QuickCommand::Compare => Some("compare.choose"),
            QuickCommand::BrowseReflog => Some("reflog.browse"),
            QuickCommand::ApplyPatch => Some("patch.apply"),
            QuickCommand::SwitchIdentity => Some("identity.switch"),
            QuickCommand::SignInToGitHub => Some("github.load-pull-requests"),
            QuickCommand::UndoLastOperation => Some("undo.last"),
            QuickCommand::SwitchWorktree => Some("worktree.switch"),
            QuickCommand::CreateWorktree => Some("worktree.create"),
            QuickCommand::ChooseAiModel => Some("ai.choose-model"),
            QuickCommand::TrustedFolders => Some("trust.manage"),
            _ => self.file_page_operation(),
        }
    }
}

/// The box of Quick search in the middle of the rectangle its caller clipped it to, as wide as it can be up to its
/// widest, and not there when the header has no room for it; with the picker over the window while it is open.
/// `Ctrl+K` (`Cmd+K` on macOS), a click, `Enter` or a key typed in the box opens the picker.
pub fn quick_search_view(state: &QuickSearchState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let opens = |typed: String| Intent::Search(SearchIntent::OpenQuick(typed));
    if !state.is_open && ui.unclaimed_keys().into_iter().any(is_quick_key) {
        intents.push(opens(String::new()));
    }
    let cell = ui.current_clip();
    let width = cell.width.min(SEARCH_WIDTH);
    if width >= ui.theme().sizes.icon * 4.0 {
        let height = ui.theme().sizes.control_height;
        let rect = place(cell, (width, height), (Align::Center, Align::Center));
        let input = InputBox::new(state.typed, header_words::QUICK_SEARCH)
            .with_icon(Icon::Search)
            .with_key_hint(header_words::QUICK_SEARCH_KEYS);
        let typed = ui.input_box("quick search", rect, &input);
        if !state.is_open {
            if let Some(text) = typed.changed_text {
                intents.push(opens(text));
            } else if typed.response.clicked || typed.is_submitted {
                intents.push(opens(String::new()));
            }
        }
    }
    if state.is_open {
        intents.extend(picker(state, ui));
    }
    intents
}

/// The picker of Quick search, and what was chosen in it.
fn picker(state: &QuickSearchState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let items: Vec<PickerItem<'_>> = state
        .entries
        .iter()
        .map(|entry| PickerItem {
            label: &entry.label,
            description: &entry.detail,
        })
        .collect();
    let picker = Picker {
        placeholder: words::QUICK_SEARCH,
        items: &items,
        mode: PickerMode::Single,
        query: state.typed,
    };
    let intent = match ui.picker("quick search picker", &picker) {
        Some(PickerAnswer::Chose(index)) => state.entries.get(index).map_or(SearchIntent::CloseQuick, |entry| {
            SearchIntent::ChooseQuick(entry.choice.clone())
        }),
        Some(PickerAnswer::Cancelled) => SearchIntent::CloseQuick,
        // A picker of one item never answers with several.
        Some(PickerAnswer::ChoseMany(_)) | None => return Vec::new(),
    };
    vec![Intent::Search(intent)]
}

/// Whether a key opens Quick search: `Ctrl+K`, or `Cmd+K` on macOS.
fn is_quick_key(key: ui::KeyEvent) -> bool {
    matches!(key.key, Key::Character('k' | 'K')) && key.modifiers == Modifiers::CONTROL
}
