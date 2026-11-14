//! The page that chooses what new worktrees get (`worktree-setup.page`): what git ignores in the folder the views show,
//! in sections, with what is proposed for each and what it gets; the link and copy patterns and the commands; and
//! where they are saved.

mod bottom;
mod lines;
mod lists;
mod rows;
mod table;

use std::collections::{BTreeMap, BTreeSet};

use model::worktree_setup::{CommandProposal, IgnoredEntry, Section, SettingsPlace, SetupPatterns, SetupSettings};
use ui::layout::{Insets, inset};
use ui::{AccessRole, Rect, Ui};

pub use lines::{Chosen, SetupLine, lines};

use crate::intent::Intent;
use crate::text::worktree_setup as words;

/// The widest the page is.
const PAGE_WIDTH: f32 = 1040.0;

/// How far what git ignores is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupRead<'a> {
    /// The first read runs: **Looking at what git ignores…**.
    Looking,
    /// Git could not say, with its message.
    Failed(&'a str),
    /// What git ignores.
    Read(&'a [IgnoredEntry]),
}

/// The three boxes of the page where something is typed and added.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SetupBox {
    /// The patterns of the folders to link.
    Link,
    /// The patterns of the files and folders to copy.
    Copy,
    /// The commands.
    Command,
}

/// What is typed in the boxes of the page.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SetupTyped {
    /// In the box of the folders to link.
    pub link: String,
    /// In the box of the files and folders to copy.
    pub copy: String,
    /// In the box of the commands.
    pub command: String,
}

impl SetupTyped {
    /// What a box holds.
    #[must_use]
    pub fn of(&self, which: SetupBox) -> &str {
        match which {
            SetupBox::Link => &self.link,
            SetupBox::Copy => &self.copy,
            SetupBox::Command => &self.command,
        }
    }
}

/// How what the page shows stands against what is saved in the place chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct SetupSaved {
    /// Whether what the page shows is what is saved there: **Save** is disabled.
    pub is_saved: bool,
    /// Whether the page has changes not saved.
    pub has_changes: bool,
    /// Whether anything of the three settings is set there: **Remove from** is there.
    pub is_set_in_place: bool,
}

/// What the page shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetupPageState<'a> {
    /// The name of the repository.
    pub repository: &'a str,
    /// What applies to new worktrees now, and where each comes from.
    pub in_effect: &'a SetupSettings,
    /// What git ignores, as far as it is read.
    pub read: SetupRead<'a>,
    /// The patterns of the page, the patterns when git was last asked, and what it said they take.
    pub chosen: Chosen<'a>,
    /// Whether git is asked again (**Looking…**).
    pub is_looking: bool,
    /// What git said when it could not say what the patterns take.
    pub look_failed: Option<&'a str>,
    /// The sections opened or closed by hand.
    pub toggled: &'a BTreeMap<Section, bool>,
    /// The names in several places unfolded, by the pattern of the name.
    pub unfolded: &'a BTreeSet<String>,
    /// The sections that list all their lines, past the first 200.
    pub shown_all: &'a BTreeSet<Section>,
    /// The commands of the page.
    pub commands: &'a [String],
    /// The commands proposed from the lock files committed here.
    pub proposals: &'a [CommandProposal],
    /// What is typed in the boxes.
    pub typed: &'a SetupTyped,
    /// Where it is saved.
    pub place: SettingsPlace,
    /// Whether VS Code's settings get the same patterns.
    pub also_vs_code: bool,
    /// How what the page shows stands against what is saved.
    pub saved: SetupSaved,
    /// How many worktrees there are besides the one the views show, not missing.
    pub other_worktrees: usize,
}

/// Something asked of the page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetupIntent {
    /// Open the page of the repository shown, or show the one there is.
    Open,
    /// A section opened or closed by a click on its line.
    ToggleSection(Section, bool),
    /// A name in several places unfolded or folded, by the pattern of the name.
    Unfold(String),
    /// A section lists all its lines.
    ShowAll(Section),
    /// The patterns of the page, as a choice, **Use what is proposed** or an edit of the lists made them.
    SetPatterns(SetupPatterns),
    /// The commands of the page, as an edit of the list made them.
    SetCommands(Vec<String>),
    /// Text typed in a box.
    Typed(SetupBox, String),
    /// **Look again**: read again what git ignores, and ask what the patterns take.
    LookAgain,
    /// Where the page saves.
    ChoosePlace(SettingsPlace),
    /// The box of VS Code's settings ticked or not.
    AlsoVsCode(bool),
    /// **Save**.
    Save,
    /// **Discard changes**.
    Discard,
}

/// The page that chooses what new worktrees get, over the rectangle its caller clipped it to: the header with what
/// applies now, what git ignores in sections with the buttons that choose what each line gets, the patterns, the
/// commands, and where they are saved.
///
/// Feature: worktree-setup.page
pub fn page(state: &SetupPageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let clip = ui.current_clip();
    let spacing = ui.theme().sizes.spacing;
    let width = PAGE_WIDTH.min(clip.width);
    let area = inset(
        Rect::new(clip.x, clip.y, width, clip.height),
        Insets::all(spacing.large + spacing.small),
    );
    let mut intents = Vec::new();
    ui.region("new worktrees", (area, AccessRole::Region, words::PAGE), |ui| {
        intents = rows::draw(state, ui, area);
    });
    intents
}
