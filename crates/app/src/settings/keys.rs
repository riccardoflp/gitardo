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
/// `graph.pageSize`.
pub const PAGE_SIZE: &str = "graph.pageSize";

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
