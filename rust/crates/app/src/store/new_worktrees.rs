//! The page that chooses what new worktrees get as the store keeps it (`worktree-setup.page`): what was read of the
//! worktree the views show, the patterns and the commands edited on the page and not saved, and the sections opened
//! or closed by hand, which stay so while the window is open, also after the page is closed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use model::contract::{FeatureId, Target};
use model::worktree_setup::{
    CommandProposal, IgnoredEntry, Section, SettingSource, SettingsInput, SettingsPlace, SetupPatterns, SetupPreview,
    SetupSave, SetupSettings, settings_in_effect,
};
use views::feature::FeatureIntent;
use views::page::Page;
use views::worktree_setup::{Chosen, SetupIntent, SetupPageState, SetupRead, SetupSaved, SetupTyped};

use super::message::{Effect, JobId, JobKind};
use super::state::State;
use crate::persisted::same_folder;
use crate::work::new_worktrees::{PlaceLists, SetupReadout, SetupRequest};

/// The page of new worktrees.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewWorktreesPage {
    /// The top folder of the worktree it shows.
    pub folder: Option<PathBuf>,
    /// Its title in the trail, **New Worktrees · `<repository>`**.
    pub title: String,
    /// The read of the repository the settings were read after; none to read them again.
    pub generation: Option<u64>,
    /// What applies to new worktrees.
    pub in_effect: SetupSettings,
    /// The patterns that apply.
    pub in_effect_patterns: SetupPatterns,
    /// What the repository's file sets.
    pub repository: PlaceLists,
    /// What the user's file sets.
    pub user: PlaceLists,
    /// What git ignores; none before the first look.
    pub ignored: Option<Result<Vec<IgnoredEntry>, String>>,
    /// The commands proposed from what is committed.
    pub proposals: Vec<CommandProposal>,
    /// The patterns git was last asked about.
    pub looked: SetupPatterns,
    /// What git said they take.
    pub preview: SetupPreview,
    /// What git said when it could not say what they take.
    pub look_failed: Option<String>,
    /// The patterns of the page, when they are not those that apply.
    pub patterns: Option<SetupPatterns>,
    /// The commands of the page, when they are not those that apply.
    pub commands: Option<Vec<String>>,
    /// Where it saves, when chosen on the page.
    pub place: Option<SettingsPlace>,
    /// Whether VS Code's settings get the same patterns.
    pub also_vs_code: bool,
    /// The sections opened or closed by hand, by the folder of the worktree.
    pub toggled: BTreeMap<PathBuf, BTreeMap<Section, bool>>,
    /// The names in several places unfolded.
    pub unfolded: BTreeSet<String>,
    /// The sections that list all their lines.
    pub shown_all: BTreeSet<Section>,
    /// What is typed in the boxes.
    pub typed: SetupTyped,
    /// The read running, and whether it looks.
    pub job: Option<(JobId, bool)>,
    /// Whether the settings are read again once the read running ends.
    pub is_read_again: bool,
}

impl Default for NewWorktreesPage {
    fn default() -> NewWorktreesPage {
        NewWorktreesPage {
            folder: None,
            title: String::new(),
            generation: None,
            in_effect: settings_in_effect(&SettingsInput::default()),
            in_effect_patterns: SetupPatterns::default(),
            repository: PlaceLists::default(),
            user: PlaceLists::default(),
            ignored: None,
            proposals: Vec::new(),
            looked: SetupPatterns::default(),
            preview: SetupPreview::default(),
            look_failed: None,
            patterns: None,
            commands: None,
            place: None,
            also_vs_code: false,
            toggled: BTreeMap::new(),
            unfolded: BTreeSet::new(),
            shown_all: BTreeSet::new(),
            typed: SetupTyped::default(),
            job: None,
            is_read_again: false,
        }
    }
}

impl NewWorktreesPage {
    /// The patterns the page shows.
    fn patterns(&self) -> &SetupPatterns {
        self.patterns.as_ref().unwrap_or(&self.in_effect_patterns)
    }

    /// The commands the page shows.
    fn commands(&self) -> &[String] {
        self.commands
            .as_deref()
            .unwrap_or(&self.in_effect.setup_commands.values)
    }

    /// Where it saves: as chosen, otherwise where the patterns that apply come from, the repository when that is VS
    /// Code's setting or nothing.
    fn place(&self) -> SettingsPlace {
        self.place.unwrap_or(match self.in_effect.link_folders.source {
            SettingSource::User => SettingsPlace::User,
            SettingSource::Repository | SettingSource::VsCode | SettingSource::Nothing => SettingsPlace::Repository,
        })
    }

    /// The edits that are what applies are no edits.
    fn forget_same(&mut self) {
        if self.patterns.as_ref() == Some(&self.in_effect_patterns) {
            self.patterns = None;
        }
        if self.commands.as_deref() == Some(self.in_effect.setup_commands.values.as_slice()) {
            self.commands = None;
        }
    }
}

/// Opens the page of the worktree the views show, or shows the one there is; opening it counts as having been offered
/// it.
pub fn open(state: &mut State) -> Vec<Effect> {
    if state.shown_read().is_none() {
        return Vec::new();
    }
    super::pages::show_page(state, Page::NewWorktrees);
    state.worktrees.setup_offered = true;
    follow(state)
}

/// Something asked of the page.
pub fn reduce(state: &mut State, intent: SetupIntent) -> Vec<Effect> {
    let page = &mut state.new_worktrees;
    match intent {
        SetupIntent::Open => return open(state),
        SetupIntent::ToggleSection(section, is_open) => {
            if let Some(folder) = page.folder.clone() {
                let _before = page.toggled.entry(folder).or_default().insert(section, is_open);
            }
        }
        SetupIntent::Unfold(pattern) => {
            if !page.unfolded.remove(&pattern) {
                let _new = page.unfolded.insert(pattern);
            }
        }
        SetupIntent::ShowAll(section) => {
            let _new = page.shown_all.insert(section);
        }
        SetupIntent::SetPatterns(patterns) => {
            page.patterns = Some(patterns);
            page.forget_same();
        }
        SetupIntent::SetCommands(commands) => {
            page.commands = Some(commands);
            page.forget_same();
        }
        SetupIntent::Typed(which, text) => match which {
            views::worktree_setup::SetupBox::Link => page.typed.link = text,
            views::worktree_setup::SetupBox::Copy => page.typed.copy = text,
            views::worktree_setup::SetupBox::Command => page.typed.command = text,
        },
        SetupIntent::LookAgain => return start(state, true),
        SetupIntent::ChoosePlace(place) => page.place = Some(place),
        SetupIntent::AlsoVsCode(is_on) => page.also_vs_code = is_on,
        SetupIntent::Save => {
            let save = SetupSave {
                place: page.place(),
                patterns: page.patterns().clone(),
                commands: page.commands().to_vec(),
                also_vs_code: page.also_vs_code && page.place() == SettingsPlace::Repository,
            };
            let run = FeatureIntent::Run(FeatureId("worktree-setup.save"), Target::SetupSettings(Box::new(save)));
            return super::feature::reduce(state, run);
        }
        SetupIntent::Discard => {
            page.patterns = None;
            page.commands = None;
            page.typed = SetupTyped::default();
        }
    }
    Vec::new()
}

/// Reads the settings again while the page is open and the repository shown was read since, or is another one; looks
/// at what git ignores the first time, and again for another worktree. Cancels the read once the page is closed.
pub fn follow(state: &mut State) -> Vec<Effect> {
    if !state.pages.open.contains(&Page::NewWorktrees) {
        return cancel(state);
    }
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let (folder, generation) = (read.state.folder.clone(), read.generation);
    let page = &mut state.new_worktrees;
    let is_same_folder = page.folder.as_deref().is_some_and(|known| same_folder(known, &folder));
    if !is_same_folder {
        let toggled = std::mem::take(&mut page.toggled);
        *page = NewWorktreesPage {
            title: views::text::worktree_setup::title(&folder_name(&folder)),
            folder: Some(folder),
            toggled,
            ..NewWorktreesPage::default()
        };
        return start(state, true);
    }
    if page.generation == Some(generation) {
        return Vec::new();
    }
    if page.job.is_some() {
        page.is_read_again = true;
        return Vec::new();
    }
    start(state, false)
}

/// The settings changed elsewhere: the page reads them again.
pub fn settings_changed(state: &mut State) {
    state.new_worktrees.generation = None;
}

/// The last name of a folder.
fn folder_name(folder: &std::path::Path) -> String {
    folder.file_name().map_or_else(
        || folder.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// Starts a read of the settings, and of what git ignores and what the patterns take when it looks; a look cancels
/// the read running.
fn start(state: &mut State, is_look: bool) -> Vec<Effect> {
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let generation = read.generation;
    let page = &mut state.new_worktrees;
    let Some(folder) = page.folder.clone() else {
        return Vec::new();
    };
    let mut effects = Vec::new();
    if let Some((job, was_look)) = page.job {
        if !is_look || was_look {
            page.is_read_again = true;
            return effects;
        }
        page.job = None;
        let _cancelled = state.jobs.remove(&job);
        effects.push(Effect::Cancel(job));
    }
    let page = &mut state.new_worktrees;
    page.generation = Some(generation);
    page.is_read_again = false;
    let request = SetupRequest {
        user_file: state.user_settings_file.clone(),
        is_look,
        patterns: page.patterns.clone(),
    };
    let job = super::workspace::start_job(state, JobKind::ReadNewWorktrees { folder, request });
    state.new_worktrees.job = Some((job.id, is_look));
    effects.push(Effect::Start(job));
    effects
}

/// Cancels the read running once the page is closed; the next opening reads again.
fn cancel(state: &mut State) -> Vec<Effect> {
    let page = &mut state.new_worktrees;
    page.folder = None;
    let Some((job, _)) = page.job.take() else {
        return Vec::new();
    };
    let _cancelled = state.jobs.remove(&job);
    vec![Effect::Cancel(job)]
}

/// Takes what was read; edits that are what applies now are forgotten, the others kept.
pub fn setup_read(state: &mut State, job: JobId, readout: SetupReadout) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    let page = &mut state.new_worktrees;
    if page.job.map(|(running, _)| running) != Some(job) {
        return Vec::new();
    }
    page.job = None;
    page.in_effect_patterns = features::worktree_setup::files::patterns_of(&readout.in_effect);
    page.in_effect = readout.in_effect;
    page.repository = readout.repository;
    page.user = readout.user;
    if let Some(look) = readout.look {
        page.ignored = Some(look.ignored);
        page.proposals = look.proposals;
        page.looked = look.patterns;
        match look.preview {
            Ok(preview) => {
                page.preview = preview;
                page.look_failed = None;
            }
            Err(error) => {
                page.preview = SetupPreview::default();
                page.look_failed = Some(error);
            }
        }
    }
    page.forget_same();
    if page.is_read_again {
        return start(state, false);
    }
    Vec::new()
}

/// How many worktrees of the repository shown there are besides the one the views show, not missing.
fn other_worktrees(state: &State) -> usize {
    let Some(read) = state.shown_read() else {
        return 0;
    };
    read.state
        .worktrees
        .iter()
        .filter(|worktree| !worktree.is_current && !worktree.is_missing)
        .filter(|worktree| !same_folder(&worktree.folder, &read.state.folder))
        .count()
}

/// No sections toggled.
static NONE_TOGGLED: BTreeMap<Section, bool> = BTreeMap::new();

/// What the page is drawn from.
#[must_use]
pub fn page_state(state: &State) -> SetupPageState<'_> {
    let page = &state.new_worktrees;
    let read = match &page.ignored {
        None => SetupRead::Looking,
        Some(Err(error)) => SetupRead::Failed(error),
        Some(Ok(entries)) => SetupRead::Read(entries),
    };
    let place = page.place();
    let in_place = match place {
        SettingsPlace::Repository => &page.repository,
        SettingsPlace::User => &page.user,
    };
    let has_changes = page.patterns.is_some() || page.commands.is_some();
    let toggled = page
        .folder
        .as_ref()
        .and_then(|folder| page.toggled.get(folder))
        .unwrap_or(&NONE_TOGGLED);
    SetupPageState {
        repository: page.folder.as_deref().map_or("", |folder| {
            folder.file_name().and_then(|name| name.to_str()).unwrap_or_default()
        }),
        in_effect: &page.in_effect,
        read,
        chosen: Chosen {
            patterns: page.patterns(),
            looked: &page.looked,
            preview: &page.preview,
        },
        is_looking: page.job.is_some_and(|(_, is_look)| is_look),
        look_failed: page.look_failed.as_deref(),
        toggled,
        unfolded: &page.unfolded,
        shown_all: &page.shown_all,
        commands: page.commands(),
        proposals: &page.proposals,
        typed: &page.typed,
        place,
        also_vs_code: page.also_vs_code,
        saved: SetupSaved {
            is_saved: in_place.sets(page.patterns(), page.commands()),
            has_changes,
            is_set_in_place: in_place.is_set(),
        },
        other_worktrees: other_worktrees(state),
    }
}
