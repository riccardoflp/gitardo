//! What each thing git ignores is, which section of the page it goes in and what is proposed for it
//! (`worktree-setup.propose`), and the commands proposed from the lock files the repository commits
//! (`worktree-setup.propose-commands`).

/// What a line of the page gets in new worktrees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Gets {
    /// A link to the folder of the checkout.
    Link,
    /// A copy.
    Copy,
    /// Nothing: the worktree makes its own, or does without.
    Nothing,
}

/// The sections of the page, in the order it lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Section {
    /// Not one Gitardo knows.
    NotKnown,
    /// Dependencies.
    Dependencies,
    /// Settings and secrets of this machine.
    Secrets,
    /// Settings of the editor.
    Editor,
    /// Build output.
    BuildOutput,
    /// Other worktrees.
    OtherWorktrees,
    /// Logs and files the system leaves.
    Logs,
}

impl Section {
    /// Every section, in the order of the page.
    pub const ALL: [Section; 7] = [
        Section::NotKnown,
        Section::Dependencies,
        Section::Secrets,
        Section::Editor,
        Section::BuildOutput,
        Section::OtherWorktrees,
        Section::Logs,
    ];
}

/// What [`propose`] looks at: something git ignores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProposeInput<'a> {
    /// Its path from the top of the checkout, with `/`.
    pub path: &'a str,
    /// Whether it is a folder (or a link to one), not a file.
    pub is_folder: bool,
}

/// What something git ignores is and what is proposed for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    /// The section it goes in.
    pub section: Section,
    /// What is proposed.
    pub proposed: Gets,
    /// The pattern a line chooses by: `node_modules/` for a folder of dependencies, wherever it is; `/.env` or
    /// `/config/` for anything else, by its path from the top.
    pub pattern: String,
}

/// The folders of dependencies, by name.
const DEPENDENCIES: [&str; 11] = [
    "node_modules",
    "bower_components",
    "jspm_packages",
    ".venv",
    "venv",
    ".pnpm-store",
    "Pods",
    "Carthage",
    "vendor",
    ".gradle",
    ".m2",
];
/// The folders of build output, by name.
const BUILD_OUTPUT: [&str; 24] = [
    "dist",
    "build",
    "out",
    "target",
    "bin",
    "obj",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".angular",
    ".turbo",
    ".parcel-cache",
    ".cache",
    "coverage",
    ".nyc_output",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".tox",
    ".dart_tool",
    "DerivedData",
    "tmp",
    "temp",
];
/// The folders of other worktrees, by name.
const OTHER_WORKTREES: [&str; 2] = ["worktrees", ".worktrees"];
/// The files the system leaves, by name, without regard to case.
const SYSTEM_FILES: [&str; 3] = [".ds_store", "thumbs.db", "desktop.ini"];
/// The endings of logs and of files editors leave, without regard to case.
const LOG_ENDINGS: [&str; 6] = [".log", ".swp", ".swo", ".tmp", ".bak", "~"];
/// The files of settings and secrets of this machine, by name, without regard to case.
const SECRET_FILES: [&str; 10] = [
    ".env",
    ".envrc",
    ".npmrc",
    ".yarnrc",
    ".yarnrc.yml",
    ".pypirc",
    ".tool-versions",
    "local.properties",
    "appsettings.development.json",
    "appsettings.local.json",
];
/// The folders of the settings of an editor, by name.
const EDITOR_FOLDERS: [&str; 4] = [".vscode", ".idea", ".fleet", ".zed"];

/// What something git ignores is, the section of the page it goes in, what is proposed for it, and the pattern it is
/// chosen by: the first rule of the card that applies.
///
/// Feature: worktree-setup.propose
#[must_use]
pub fn propose(input: &ProposeInput<'_>) -> Proposal {
    let path = input.path.trim_matches('/');
    let name = path.rsplit('/').next().unwrap_or(path);
    let lower = name.to_lowercase();
    let by_path = if input.is_folder {
        format!("/{path}/")
    } else {
        format!("/{path}")
    };
    let proposal = |section, proposed, pattern| Proposal {
        section,
        proposed,
        pattern,
    };
    if input.is_folder && DEPENDENCIES.contains(&name) {
        return proposal(Section::Dependencies, Gets::Link, format!("{name}/"));
    }
    if input.is_folder && (path == ".yarn/cache" || path.ends_with("/.yarn/cache")) {
        return proposal(Section::Dependencies, Gets::Link, by_path);
    }
    let section = if input.is_folder && BUILD_OUTPUT.contains(&name) {
        Section::BuildOutput
    } else if input.is_folder && OTHER_WORKTREES.contains(&name) {
        Section::OtherWorktrees
    } else if SYSTEM_FILES.contains(&lower.as_str()) || LOG_ENDINGS.iter().any(|ending| lower.ends_with(ending)) {
        Section::Logs
    } else if !input.is_folder && is_secret(&lower) {
        Section::Secrets
    } else if input.is_folder && EDITOR_FOLDERS.contains(&name) {
        Section::Editor
    } else {
        Section::NotKnown
    };
    let proposed = match section {
        Section::Secrets | Section::Editor => Gets::Copy,
        Section::Dependencies => Gets::Link,
        Section::NotKnown | Section::BuildOutput | Section::OtherWorktrees | Section::Logs => Gets::Nothing,
    };
    proposal(section, proposed, by_path)
}

/// Whether a file, by its name in lower case, holds settings or secrets of this machine: one of the names of the card,
/// `.env.<anything>`, `<name>.local` or `<name>.local.<extension>`.
fn is_secret(name: &str) -> bool {
    if SECRET_FILES.contains(&name) || name.strip_prefix(".env.").is_some_and(|rest| !rest.is_empty()) {
        return true;
    }
    let mut parts = name.rsplitn(3, '.');
    let (last, middle, first) = (parts.next(), parts.next(), parts.next());
    match (last, middle, first) {
        (Some("local"), Some(stem), _) => !stem.is_empty(),
        (Some(extension), Some("local"), Some(stem)) => !extension.is_empty() && !stem.is_empty(),
        _ => false,
    }
}

/// What [`propose_commands`] looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CommandsInput<'a> {
    /// The files committed at the top of the repository, among the lock files of the card.
    pub committed: &'a [String],
    /// `packageManager` of `package.json`, when it says one: `pnpm@9.0.0`.
    pub package_manager: Option<&'a str>,
}

/// A command proposed for new worktrees, and the file it is proposed for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandProposal {
    /// The command: `npm ci`.
    pub command: String,
    /// The lock file it is proposed for: `package-lock.json`.
    pub file: String,
}

/// The files whose being committed at the top the proposals look at, for [`propose_commands`].
pub const LOCK_FILES: [&str; 13] = [
    "pnpm-lock.yaml",
    "yarn.lock",
    ".yarnrc.yml",
    "bun.lock",
    "bun.lockb",
    "package-lock.json",
    "npm-shrinkwrap.json",
    "package.json",
    "uv.lock",
    "poetry.lock",
    "Cargo.lock",
    "go.mod",
    "Gemfile.lock",
];

/// The commands proposed from the lock files committed at the top: one for JavaScript, one for Python, and those of
/// Cargo, Go and Bundler, each with the file it is for.
///
/// Feature: worktree-setup.propose-commands
#[must_use]
pub fn propose_commands(input: &CommandsInput<'_>) -> Vec<CommandProposal> {
    let has = |file: &str| input.committed.iter().any(|committed| committed == file);
    let proposal = |command: &str, file: &str| CommandProposal {
        command: command.to_owned(),
        file: file.to_owned(),
    };
    let mut proposals = Vec::new();
    proposals.extend(javascript(input, &has));
    if has("uv.lock") {
        proposals.push(proposal("uv sync --locked", "uv.lock"));
    } else if has("poetry.lock") {
        proposals.push(proposal("poetry install", "poetry.lock"));
    }
    for (file, command) in [
        ("Cargo.lock", "cargo fetch"),
        ("go.mod", "go mod download"),
        ("Gemfile.lock", "bundle install"),
    ] {
        if has(file) {
            proposals.push(proposal(command, file));
        }
    }
    proposals
}

/// The JavaScript managers, in the order the card tries them, each with its lock files.
const MANAGERS: [(&str, &[&str]); 4] = [
    ("pnpm", &["pnpm-lock.yaml"]),
    ("yarn", &["yarn.lock"]),
    ("bun", &["bun.lock", "bun.lockb"]),
    ("npm", &["package-lock.json", "npm-shrinkwrap.json"]),
];

/// The command for JavaScript: that of the manager `packageManager` names when its lock file is committed, otherwise
/// that of the first manager whose lock file is.
fn javascript(input: &CommandsInput<'_>, has: &dyn Fn(&str) -> bool) -> Option<CommandProposal> {
    let named = input
        .package_manager
        .map(|manager| manager.split('@').next().unwrap_or(manager).trim());
    let lock_of = |manager: &str| {
        MANAGERS
            .iter()
            .find(|(name, _)| *name == manager)
            .and_then(|(_, files)| files.iter().find(|file| has(file)).copied())
    };
    let (manager, file) = named
        .and_then(|manager| lock_of(manager).map(|file| (manager, file)))
        .or_else(|| {
            MANAGERS
                .iter()
                .find_map(|(manager, _)| lock_of(manager).map(|file| (*manager, file)))
        })?;
    let command = match manager {
        "pnpm" => "pnpm install --frozen-lockfile",
        "yarn" if has(".yarnrc.yml") || is_modern_yarn(input.package_manager) => "yarn install --immutable",
        "yarn" => "yarn install --frozen-lockfile",
        "bun" => "bun install",
        _ => "npm ci", // The last of the managers the card names.
    };
    Some(CommandProposal {
        command: command.to_owned(),
        file: file.to_owned(),
    })
}

/// Whether `packageManager` names Yarn 2 or later: `yarn@4.1.0`.
fn is_modern_yarn(package_manager: Option<&str>) -> bool {
    package_manager
        .and_then(|manager| manager.trim().strip_prefix("yarn@"))
        .and_then(|version| version.split('.').next())
        .and_then(|major| major.parse::<u32>().ok())
        .is_some_and(|major| major >= 2)
}

#[cfg(test)]
mod tests;
