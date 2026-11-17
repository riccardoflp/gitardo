//! The words of the settings page (`settings.edit`) and of the picker of the themes (`settings.theme`): every setting of
//! `docs/features/18-settings.md`, with its description, its default and the files it may be set in, as the card lists
//! them.

/// What a setting holds, which says how the settings page changes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingKind {
    /// `true` or `false`, changed with a switch.
    Toggle,
    /// One of some words, changed by choosing one.
    Choice(&'static [&'static str]),
    /// A number.
    Number,
    /// A string.
    Text,
    /// A list.
    List,
}

/// A setting, as the settings page lists it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingInfo {
    /// Its key: `graph.pageSize`.
    pub key: &'static str,
    /// What it does.
    pub description: &'static str,
    /// Its default, as written in a settings file.
    pub default: &'static str,
    /// Whether the repository's file may set it too.
    pub is_repository: bool,
    /// Whether it is experimental.
    pub is_experimental: bool,
    /// What it holds.
    pub kind: SettingKind,
}

/// Every setting, in the order of the card.
pub const SETTINGS: [SettingInfo; 51] = [
    SettingInfo {
        key: "commandPreview",
        description: "When the commands are shown before they run: off for the ones that can lose work, all for everything that changes something.",
        default: "\"off\"",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Choice(&["off", "all"]),
    },
    SettingInfo {
        key: "identities",
        description: "Names, emails and optional signing keys to choose from when switching identity.",
        default: "[]",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::List,
    },
    SettingInfo {
        key: "readOnStartup",
        description: "Read the repository when the window opens, before the timeline is shown.",
        default: "true",
        is_repository: true,
        is_experimental: false,
        kind: SettingKind::Toggle,
    },
    SettingInfo {
        key: "repositories.keepOpen",
        description: "Each repository keeps what it showed while another is shown, and the tabs not shown are read in the background.",
        default: "true",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Toggle,
    },
    SettingInfo {
        key: "graph.pageSize",
        description: "How many commits the timeline reads at a time.",
        default: "1000",
        is_repository: true,
        is_experimental: false,
        kind: SettingKind::Number,
    },
    SettingInfo {
        key: "graph.showStashes",
        description: "Show the stashes in the timeline.",
        default: "true",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Toggle,
    },
    SettingInfo {
        key: "graph.columns",
        description: "What is shown beside the message of each commit.",
        default: "[\"author\",\"date\",\"hash\"]",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::List,
    },
    SettingInfo {
        key: "graph.dateSeparators",
        description: "The lines of yesterday, this week, last week and each month.",
        default: "true",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Toggle,
    },
    SettingInfo {
        key: "graph.combineUpstream",
        description: "A branch and its upstream of the same name share one label while at the same commit.",
        default: "true",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Toggle,
    },
    SettingInfo {
        key: "graph.firstParent",
        description: "Follow only the first parent of merges.",
        default: "false",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Toggle,
    },
    SettingInfo {
        key: "graph.traceSelected",
        description: "Bring out the history of the selected commit.",
        default: "true",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Toggle,
    },
    SettingInfo {
        key: "branches.sortOrder",
        description: "name or date, for branches, remote branches and tags everywhere they are listed. Not set: by name, or by date when Git's branch.sort is -committerdate.",
        default: "not set",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Choice(&["name", "date"]),
    },
    SettingInfo {
        key: "branches.followGraphFilter",
        description: "Not read any more: the branch rail has a filter of its own.",
        default: "true",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Toggle,
    },
    SettingInfo {
        key: "branches.format",
        description: "A format for the names of new branches and worktrees.",
        default: "\"\"",
        is_repository: true,
        is_experimental: false,
        kind: SettingKind::Text,
    },
    SettingInfo {
        key: "branches.inputs",
        description: "What the ${input:<id>} of the format asks for.",
        default: "[]",
        is_repository: true,
        is_experimental: false,
        kind: SettingKind::List,
    },
    SettingInfo {
        key: "branches.openIn",
        description: "What a double click on a local branch does: checkout or worktree.",
        default: "\"checkout\"",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Choice(&["checkout", "worktree"]),
    },
    SettingInfo {
        key: "pull.rebase",
        description: "Pull adds --rebase.",
        default: "true",
        is_repository: true,
        is_experimental: false,
        kind: SettingKind::Toggle,
    },
    SettingInfo {
        key: "pull.autostash",
        description: "Pull adds --autostash.",
        default: "true",
        is_repository: true,
        is_experimental: false,
        kind: SettingKind::Toggle,
    },
    SettingInfo {
        key: "cherryPick.recordOrigin",
        description: "Cherry-picks end their message with (cherry picked from commit \u{2026}), as -x does.",
        default: "false",
        is_repository: true,
        is_experimental: false,
        kind: SettingKind::Toggle,
    },
    SettingInfo {
        key: "submodules.updateOnCheckout",
        description: "After a checkout, update the submodules to the commits it records.",
        default: "true",
        is_repository: true,
        is_experimental: false,
        kind: SettingKind::Toggle,
    },
    SettingInfo {
        key: "changes.openOnClick",
        description: "What a click on a file of the Changes page opens: file or diff.",
        default: "\"file\"",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Choice(&["file", "diff"]),
    },
    SettingInfo {
        key: "worktrees.openIn",
        description: "Where a worktree opens: window (a new window of Gitardo Desktop), editor (the external editor) or terminal.",
        default: "\"window\"",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Choice(&["window", "editor", "terminal"]),
    },
    SettingInfo {
        key: "worktrees.folder",
        description: "Where new worktrees go.",
        default: "\"${repo}.worktrees\"",
        is_repository: true,
        is_experimental: false,
        kind: SettingKind::Text,
    },
    SettingInfo {
        key: "worktrees.linkFolders",
        description: "Ignored folders to link into new worktrees, in .gitignore syntax.",
        default: "[]",
        is_repository: true,
        is_experimental: false,
        kind: SettingKind::List,
    },
    SettingInfo {
        key: "worktrees.copyFiles",
        description: "Ignored files and folders to copy into new worktrees, in .gitignore syntax.",
        default: "[]",
        is_repository: true,
        is_experimental: false,
        kind: SettingKind::List,
    },
    SettingInfo {
        key: "worktrees.setupCommands",
        description: "Commands to run in each new worktree, in Tasks of the Activity drawer, in a trusted folder only.",
        default: "[]",
        is_repository: true,
        is_experimental: false,
        kind: SettingKind::List,
    },
    SettingInfo {
        key: "conflicts.autoResolve",
        description: "Offer to resolve the conflicts that need no choice.",
        default: "false",
        is_repository: false,
        is_experimental: true,
        kind: SettingKind::Toggle,
    },
    SettingInfo {
        key: "conflicts.ai",
        description: "Offer to ask the AI provider about the conflicts the rules leave.",
        default: "false",
        is_repository: false,
        is_experimental: true,
        kind: SettingKind::Toggle,
    },
    SettingInfo {
        key: "blame.enabled",
        description: "The blame of the line under the cursor, at its end, in the File page.",
        default: "true",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Toggle,
    },
    SettingInfo {
        key: "blame.ignoreWhitespaceAndMoves",
        description: "Blame with -w -M.",
        default: "false",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Toggle,
    },
    SettingInfo {
        key: "fileBlame.everyFile",
        description: "Show the blame beside the lines of every file opened.",
        default: "false",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Toggle,
    },
    SettingInfo {
        key: "fileBlame.format",
        description: "What the blame says beside each line.",
        default: "\"${date} ${author:20}\"",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Text,
    },
    SettingInfo {
        key: "fileBlame.groupLines",
        description: "Say the commit once for the lines of it that follow each other.",
        default: "true",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Toggle,
    },
    SettingInfo {
        key: "fileBlame.age.enabled",
        description: "The bar in the color of the age of each line.",
        default: "true",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Toggle,
    },
    SettingInfo {
        key: "fileBlame.age.recentColor",
        description: "The color of a line just changed.",
        default: "\"#e5534b\"",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Text,
    },
    SettingInfo {
        key: "fileBlame.age.oldColor",
        description: "The color of a line changed long ago.",
        default: "\"#316dca\"",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Text,
    },
    SettingInfo {
        key: "fileBlame.age.oldAfterDays",
        description: "After how many days a line has the old color.",
        default: "730",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Number,
    },
    SettingInfo {
        key: "diff.collapseUnchanged",
        description: "The Diff page folds long runs of unchanged lines, keeping 3 around each change. Was flowDiff.collapseUnchanged.",
        default: "true",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Toggle,
    },
    SettingInfo {
        key: "ai.provider",
        description: "The kind of HTTP interface of the language model: anthropic or openai-compatible. Empty: no AI.",
        default: "\"\"",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Text,
    },
    SettingInfo {
        key: "ai.endpoint",
        description: "The base URL of the provider; empty for the provider's own.",
        default: "\"\"",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Text,
    },
    SettingInfo {
        key: "ai.model",
        description: "The model asked, as the provider names it. Was conflicts.aiModel, now used for drafting commit messages too.",
        default: "\"\"",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Text,
    },
    SettingInfo {
        key: "ai.effort",
        description: "How much the model thinks before it answers: empty, low, medium, high, xhigh or max. Was conflicts.aiEffort.",
        default: "\"\"",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Text,
    },
    SettingInfo {
        key: "editor.external",
        description: "The command that opens a file in another editor, with ${file} and ${line}, such as code --goto ${file}:${line}. Empty: the system's default program for the file.",
        default: "\"\"",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Text,
    },
    SettingInfo {
        key: "terminal.external",
        description: "The command that opens a terminal in a folder, with ${folder}. Empty: the system's default terminal.",
        default: "\"\"",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Text,
    },
    SettingInfo {
        key: "terminal.shellPath",
        description: "The shell the terminals of the bottom panel start. Empty: PowerShell 7 on Windows when it is installed, else Windows PowerShell; the shell of SHELL on Linux and macOS.",
        default: "\"\"",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Text,
    },
    SettingInfo {
        key: "terminal.scrollbackLines",
        description: "How many lines a terminal keeps above its screen.",
        default: "10000",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Number,
    },
    SettingInfo {
        key: "theme",
        description: "system, dark, light, highContrastDark or highContrastLight.",
        default: "\"system\"",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Choice(&["system", "dark", "light", "highContrastDark", "highContrastLight"]),
    },
    SettingInfo {
        key: "font.ui",
        description: "The families and the size of the text of the interface and of code: the fonts that come with the program, or the name of a font of the system.",
        default: "\"Inter\"",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Text,
    },
    SettingInfo {
        key: "font.code",
        description: "The families and the size of the text of the interface and of code: the fonts that come with the program, or the name of a font of the system.",
        default: "\"JetBrains Mono\"",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Text,
    },
    SettingInfo {
        key: "font.size",
        description: "The families and the size of the text of the interface and of code: the fonts that come with the program, or the name of a font of the system.",
        default: "13",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Number,
    },
    SettingInfo {
        key: "log.level",
        description: "info logs what changes something; debug logs the reads too.",
        default: "\"info\"",
        is_repository: false,
        is_experimental: false,
        kind: SettingKind::Choice(&["info", "debug"]),
    },
];

/// The title of the settings page, in its trail.
pub const PAGE: &str = "Settings";
/// Said beside the name of an experimental setting.
pub const EXPERIMENTAL: &str = "Experimental";
/// Where a value comes from: the default.
pub const FROM_DEFAULT: &str = "Default";
/// Where a value comes from: the user's file.
pub const FROM_USER: &str = "User settings";
/// Where a value comes from: the repository's file.
pub const FROM_REPOSITORY: &str = "Repository settings";
/// The label of the choice of the file the page writes in.
pub const WRITE_IN: &str = "Write changes in";
/// The button that opens the settings file in a File page.
pub const EDIT_IN_FILE: &str = "Edit in settings.json";
/// The tooltip of the control of a setting the repository's file may not set, while changes go to that file.
pub const USER_ONLY: &str = "Only the user settings can set this";
/// The picker of the themes (`settings.theme`).
pub const CHOOSE_THEME: &str = "Choose the theme";
/// **Theme…** in the **…** menu.
pub const THEME_ENTRY: &str = "Theme\u{2026}";
/// **Settings** in the **…** menu.
pub const SETTINGS_ENTRY: &str = "Settings";
/// The themes, in the order of the values of `theme`, as the picker names them.
pub const THEME_NAMES: [&str; 5] = [
    "Follow the system",
    "Dark",
    "Light",
    "High contrast dark",
    "High contrast light",
];
/// The values of `theme`, in the order of [`THEME_NAMES`].
pub const THEME_VALUES: [&str; 5] = ["system", "dark", "light", "highContrastDark", "highContrastLight"];
/// What the picker says of the theme in force.
pub const CURRENT_THEME: &str = "current";

/// Where a value comes from, said under it.
#[must_use]
pub fn source(file: &str) -> String {
    format!("From {}", file.to_lowercase())
}

/// A value not taken from a settings file, said under its setting: the file, then why.
#[must_use]
pub fn problem(file: &str, reason: &str) -> String {
    format!("{file}: {reason}. The default is used.")
}
