//! What the page that chooses what new worktrees get reads on a worker (`worktree-setup.page`): the three settings
//! files and what applies from them, and, when it opens and at **Look again**, what git ignores, the commands proposed
//! from what is committed, and what the patterns take.

use std::path::{Path, PathBuf};

use features::worktree_setup::files::{COPY_FILES, LINK_FOLDERS, SETUP_COMMANDS, SettingsFile, SettingsFiles};
use git::read::worktree_setup::{
    IgnoredRequest, PatternsRequest, read_committed_at_top, read_ignored, read_what_patterns_take,
};
use git::{CancelToken, Git};
use model::worktree_setup::{
    CommandProposal, CommandsInput, IgnoredEntry, LOCK_FILES, SettingsPlace, SetupPatterns, SetupPreview,
    SetupSettings, propose_commands,
};

/// What the page asks to read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupRequest {
    /// The user's settings file, when the system says where it is.
    pub user_file: Option<PathBuf>,
    /// Whether git is asked what it ignores, what is committed and what the patterns take, besides the settings.
    pub is_look: bool,
    /// The patterns git is asked about: those of the page, or those that apply when the page has none of its own.
    pub patterns: Option<SetupPatterns>,
}

/// What one place sets of the three settings.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PlaceLists {
    /// `worktrees.linkFolders`.
    pub link: Option<Vec<String>>,
    /// `worktrees.copyFiles`.
    pub copy: Option<Vec<String>>,
    /// `worktrees.setupCommands`.
    pub commands: Option<Vec<String>>,
}

impl PlaceLists {
    /// What a settings file sets.
    fn of(file: &SettingsFile) -> PlaceLists {
        PlaceLists {
            link: file.list(LINK_FOLDERS),
            copy: file.list(COPY_FILES),
            commands: file.list(SETUP_COMMANDS),
        }
    }

    /// Whether it sets any of the three.
    #[must_use]
    pub fn is_set(&self) -> bool {
        self.link.is_some() || self.copy.is_some() || self.commands.is_some()
    }

    /// Whether it sets these patterns and commands, a list it does not set counting as empty.
    #[must_use]
    pub fn sets(&self, patterns: &SetupPatterns, commands: &[String]) -> bool {
        let same = |set: &Option<Vec<String>>, shown: &[String]| set.as_deref().unwrap_or_default() == shown;
        same(&self.link, &patterns.link) && same(&self.copy, &patterns.copy) && same(&self.commands, commands)
    }
}

/// What git said when it was asked: what it ignores, the commands proposed, and what the patterns take.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupLook {
    /// What git ignores, or what it said when it could not list it.
    pub ignored: Result<Vec<IgnoredEntry>, String>,
    /// The commands proposed from the lock files committed at the top.
    pub proposals: Vec<CommandProposal>,
    /// The patterns it was asked about.
    pub patterns: SetupPatterns,
    /// What they take, or what git said when it could not say.
    pub preview: Result<SetupPreview, String>,
}

/// What a read of the page gives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupReadout {
    /// What applies to new worktrees, and where each comes from.
    pub in_effect: SetupSettings,
    /// What the repository's file sets.
    pub repository: PlaceLists,
    /// What the user's file sets.
    pub user: PlaceLists,
    /// What git said, when it was asked.
    pub look: Option<SetupLook>,
}

impl SetupReadout {
    /// What a place sets.
    #[must_use]
    pub fn of(&self, place: SettingsPlace) -> &PlaceLists {
        match place {
            SettingsPlace::Repository => &self.repository,
            SettingsPlace::User => &self.user,
        }
    }
}

/// A settings file as it is on disk; none when it is not there.
fn file(path: PathBuf) -> SettingsFile {
    let text = std::fs::read(&path)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
    SettingsFile { path, text }
}

/// Reads what the page shows of a checkout.
#[must_use]
pub fn read_setup(folder: &Path, request: &SetupRequest, cancel: &CancelToken) -> SetupReadout {
    let files = SettingsFiles {
        repository: file(folder.join(".gitardo").join("settings.json")),
        user: request.user_file.clone().map(file),
        vs_code: file(folder.join(".vscode").join("settings.json")),
    };
    let in_effect = files.in_effect();
    let user = files.user.as_ref().map_or_default(PlaceLists::of);
    let look = request.is_look.then(|| {
        let patterns = request
            .patterns
            .clone()
            .unwrap_or_else(|| features::worktree_setup::files::patterns_of(&in_effect));
        look(folder, patterns, cancel)
    });
    SetupReadout {
        repository: PlaceLists::of(&files.repository),
        user,
        in_effect,
        look,
    }
}

/// Asks git what it ignores, what is committed at the top, and what the patterns take.
fn look(folder: &Path, patterns: SetupPatterns, cancel: &CancelToken) -> SetupLook {
    let git = Git::new(folder).with_cancel(cancel.clone());
    let ignored = read_ignored(
        &git,
        &IgnoredRequest {
            folder: folder.to_path_buf(),
        },
    )
    .map_err(|error| error.to_string());
    let committed = read_committed_at_top(&git, &LOCK_FILES).unwrap_or_default();
    let package_manager = committed
        .iter()
        .any(|file| file == "package.json")
        .then(|| package_manager(folder))
        .flatten();
    let proposals = propose_commands(&CommandsInput {
        committed: &committed,
        package_manager: package_manager.as_deref(),
    });
    let preview = read_what_patterns_take(
        &git,
        &PatternsRequest {
            folder: folder.to_path_buf(),
            link_patterns: patterns.link.clone(),
            copy_patterns: patterns.copy.clone(),
            commit: None,
        },
    )
    .map_err(|error| error.to_string());
    SetupLook {
        ignored,
        proposals,
        patterns,
        preview,
    }
}

/// The `packageManager` of the `package.json` at the top of the checkout.
fn package_manager(folder: &Path) -> Option<String> {
    let text = std::fs::read_to_string(folder.join("package.json")).ok()?;
    let value = json::parse_with_comments(&text).ok()?;
    value.get("packageManager")?.as_str().map(str::to_owned)
}
