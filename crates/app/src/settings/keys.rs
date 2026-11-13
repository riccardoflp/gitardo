//! The keys of the settings read here, with their defaults and the files they may be set in, as
//! `docs/features/18-settings.md` lists them.

/// `theme`.
pub const THEME: &str = "theme";
/// The values `theme` takes, in the order of [`render::ThemeName`]'s themes: the system's first.
pub const THEMES: [&str; 5] = ["system", "dark", "light", "highContrastDark", "highContrastLight"];
/// `font.ui`.
pub const FONT_INTERFACE: &str = "font.ui";
/// `font.code`.
pub const FONT_CODE: &str = "font.code";
/// `font.size`.
pub const FONT_SIZE_KEY: &str = "font.size";
/// `readOnStartup`.
pub const READ_ON_STARTUP: &str = "readOnStartup";
/// `conflicts.autoResolve`.
pub const AUTO_RESOLVE: &str = "conflicts.autoResolve";
/// `conflicts.ai`.
pub const CONFLICTS_AI: &str = "conflicts.ai";
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
/// `branches.followGraphFilter`.
pub const FOLLOW_GRAPH_FILTER: &str = "branches.followGraphFilter";
/// `commandPreview`.
pub const COMMAND_PREVIEW: &str = "commandPreview";
/// The values of `commandPreview`: the commands shown only before what can lose work, or before everything.
pub const COMMAND_PREVIEWS: [&str; 2] = ["off", "all"];
/// `branches.format`.
pub const BRANCH_FORMAT: &str = "branches.format";
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
/// `identities`.
pub const IDENTITIES: &str = "identities";
/// `cherryPick.recordOrigin`.
pub const CHERRY_PICK_RECORD_ORIGIN: &str = "cherryPick.recordOrigin";
/// `conflicts.autoResolve`.
pub const CONFLICTS_AUTO_RESOLVE: &str = "conflicts.autoResolve";
/// `conflicts.ai`.
pub const CONFLICTS_AI: &str = "conflicts.ai";

/// The default of `font.ui`: the font of the interface that comes with the program.
pub const INTERFACE_FONT: &str = "Inter";
/// The default of `font.code`: the font of code that comes with the program.
pub const CODE_FONT: &str = "JetBrains Mono";
/// The default of `font.size`.
pub const FONT_SIZE: f32 = 13.0;
/// The smallest and the largest `font.size` taken: text smaller cannot be read, larger does not fit the rows.
pub const FONT_SIZES: (f64, f64) = (8.0, 24.0);

/// Which files a setting may be set in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// The user's file only.
    User,
    /// The user's file and the repository's, which comes first.
    UserAndRepository,
}
