//! The keys of the settings read here, with their defaults and the files they may be set in, as
//! `docs/features/18-settings.md` lists them.

/// `theme`.
pub const THEME: &str = "theme";
/// The values `theme` takes, in the order of the themes of `render`: the system's first.
pub const THEMES: [&str; 5] = ["system", "dark", "light", "highContrastDark", "highContrastLight"];
/// `fileIcons`.
pub const FILE_ICONS: &str = "fileIcons";
/// The values of `fileIcons`: the icons of Symbols in the colors of the theme, the same in the color of the text, or
/// the plain icons of a file and a folder.
pub const FILE_ICON_STYLES: [&str; 3] = ["symbols", "monochrome", "simple"];
/// `font.ui`.
pub const FONT_INTERFACE: &str = "font.ui";
/// `font.code`.
pub const FONT_CODE: &str = "font.code";
/// `font.size`.
pub const FONT_SIZE_KEY: &str = "font.size";
/// `readOnStartup`.
pub const READ_ON_STARTUP: &str = "readOnStartup";
/// `repositories.keepOpen`.
pub const KEEP_OPEN: &str = "repositories.keepOpen";
/// `graph.pageSize`.
pub const PAGE_SIZE: &str = "graph.pageSize";
/// `graph.showStashes`.
pub const SHOW_STASHES: &str = "graph.showStashes";
/// `graph.columns`.
pub const COLUMNS: &str = "graph.columns";
/// The values of `graph.columns`.
pub const COLUMN_NAMES: [&str; 3] = ["author", "date", "hash"];
/// `graph.dateSeparators`.
pub const DATE_SEPARATORS: &str = "graph.dateSeparators";
/// `graph.combineUpstream`.
pub const COMBINE_UPSTREAM: &str = "graph.combineUpstream";
/// `graph.firstParent`.
pub const FIRST_PARENT: &str = "graph.firstParent";
/// `graph.oneLane`.
pub const ONE_LANE: &str = "graph.oneLane";
/// `graph.compactRows`.
pub const COMPACT_ROWS: &str = "graph.compactRows";
/// `log.level`.
pub const LOG_LEVEL: &str = "log.level";
/// The values `log.level` takes: what changes something, or the reads too.
pub const LOG_LEVELS: [&str; 2] = ["info", "debug"];
/// `graph.traceSelected`.
pub const TRACE_SELECTED: &str = "graph.traceSelected";
/// `branches.sortOrder`.
pub const SORT_ORDER: &str = "branches.sortOrder";
/// The values of `branches.sortOrder`.
pub const SORT_ORDERS: [&str; 2] = ["name", "date"];
/// `commandPreview`.
pub const COMMAND_PREVIEW: &str = "commandPreview";
/// The values of `commandPreview`: the commands shown only before what can lose work, or before everything.
pub const COMMAND_PREVIEWS: [&str; 2] = ["off", "all"];
/// `branches.format`.
pub const BRANCH_FORMAT: &str = "branches.format";
/// `branches.inputs`.
pub const BRANCH_INPUTS: &str = "branches.inputs";
/// `diff.collapseUnchanged`.
pub const COLLAPSE_UNCHANGED: &str = "diff.collapseUnchanged";
/// `changes.openOnClick`.
pub const OPEN_ON_CLICK: &str = "changes.openOnClick";
/// The values of `changes.openOnClick`.
pub const OPEN_ON_CLICKS: [&str; 2] = ["file", "diff"];
/// `submodules.updateOnCheckout`.
pub const UPDATE_SUBMODULES_ON_CHECKOUT: &str = "submodules.updateOnCheckout";
/// `branches.openIn`.
pub const BRANCH_OPEN_IN: &str = "branches.openIn";
/// The values of `branches.openIn`: what a double click on a branch does.
pub const BRANCH_OPEN_INS: [&str; 2] = ["checkout", "worktree"];
/// `pull.rebase`.
pub const PULL_REBASE: &str = "pull.rebase";
/// `pull.autostash`.
pub const PULL_AUTOSTASH: &str = "pull.autostash";
/// `rebase.updateRefs`.
pub const REBASE_UPDATE_REFS: &str = "rebase.updateRefs";
/// `identities`.
pub const IDENTITIES: &str = "identities";
/// `cherryPick.recordOrigin`.
pub const CHERRY_PICK_RECORD_ORIGIN: &str = "cherryPick.recordOrigin";
/// `commit.signOff`.
pub const COMMIT_SIGN_OFF: &str = "commit.signOff";
/// `conflicts.autoResolve`.
pub const CONFLICTS_AUTO_RESOLVE: &str = "conflicts.autoResolve";
/// `conflicts.ai`.
pub const CONFLICTS_AI: &str = "conflicts.ai";
/// `ai.provider`.
pub const AI_PROVIDER: &str = "ai.provider";
/// `ai.endpoint`.
pub const AI_ENDPOINT: &str = "ai.endpoint";
/// `ai.model`.
pub const AI_MODEL: &str = "ai.model";
/// `ai.effort`.
pub const AI_EFFORT: &str = "ai.effort";
/// `editor.external`.
pub const EXTERNAL_EDITOR: &str = "editor.external";
/// `editor.wordWrap`.
pub const WORD_WRAP: &str = "editor.wordWrap";
/// The values of `editor.wordWrap`: lines cut at the right edge, or wrapped at the width of the page.
pub const WORD_WRAPS: [&str; 2] = ["off", "on"];
/// `markdown.openAsPreview`.
pub const MARKDOWN_OPEN_AS_PREVIEW: &str = "markdown.openAsPreview";
/// `blame.enabled`.
pub const BLAME_ENABLED: &str = "blame.enabled";
/// `blame.ignoreWhitespaceAndMoves`.
pub const BLAME_IGNORE_WHITESPACE: &str = "blame.ignoreWhitespaceAndMoves";
/// `fileBlame.everyFile`.
pub const FILE_BLAME_EVERY_FILE: &str = "fileBlame.everyFile";
/// `fileBlame.format`.
pub const FILE_BLAME_FORMAT: &str = "fileBlame.format";
/// `fileBlame.groupLines`.
pub const FILE_BLAME_GROUP_LINES: &str = "fileBlame.groupLines";
/// `fileBlame.age.enabled`.
pub const FILE_BLAME_AGE_ENABLED: &str = "fileBlame.age.enabled";
/// `fileBlame.age.recentColor`.
pub const FILE_BLAME_AGE_RECENT_COLOR: &str = "fileBlame.age.recentColor";
/// `fileBlame.age.oldColor`.
pub const FILE_BLAME_AGE_OLD_COLOR: &str = "fileBlame.age.oldColor";
/// `fileBlame.age.oldAfterDays`.
pub const FILE_BLAME_AGE_OLD_AFTER_DAYS: &str = "fileBlame.age.oldAfterDays";
/// `terminal.external`.
pub const EXTERNAL_TERMINAL: &str = "terminal.external";
/// `terminal.shellPath`.
pub const TERMINAL_SHELL_PATH: &str = "terminal.shellPath";
/// `terminal.scrollbackLines`.
pub const TERMINAL_SCROLLBACK: &str = "terminal.scrollbackLines";
/// `worktrees.folder`.
pub const WORKTREES_FOLDER: &str = "worktrees.folder";
/// `worktrees.openIn`.
pub const WORKTREES_OPEN_IN: &str = "worktrees.openIn";
/// `hosting.selfHosted`.
pub const SELF_HOSTED: &str = "hosting.selfHosted";
/// The values of `worktrees.openIn`: where a worktree opens.
pub const WORKTREES_OPEN_INS: [&str; 3] = ["window", "editor", "terminal"];

/// The default of `font.ui`: the font of the interface that comes with the program.
pub const INTERFACE_FONT: &str = "Inter";
/// The default of `font.code`: the font of code that comes with the program.
pub const CODE_FONT: &str = "JetBrains Mono";
/// The default of `font.size`.
pub const FONT_SIZE: f32 = 13.0;
/// The smallest and the largest `font.size` taken: text smaller cannot be read, larger does not fit the rows.
pub const FONT_SIZES: (f64, f64) = (8.0, 24.0);
/// The default of `terminal.scrollbackLines`.
pub const SCROLLBACK_LINES: usize = 10_000;
/// The most `terminal.scrollbackLines` takes: more would hold more memory than a terminal is worth.
pub const SCROLLBACK_MOST: f64 = 1_000_000.0;

/// Which files a setting may be set in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// The user's file only.
    User,
    /// The user's file and the repository's, which comes first.
    UserAndRepository,
}
