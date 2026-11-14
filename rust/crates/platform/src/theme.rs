//! The colors the person chose for the whole system.

/// Dark or light.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Appearance {
    /// Dark text on light backgrounds.
    #[default]
    Light,
    /// Light text on dark backgrounds.
    Dark,
}

/// The theme of the system, as the person set it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SystemTheme {
    /// Whether applications should be dark or light.
    pub appearance: Appearance,
    /// Whether high contrast is on: the program should then use the high contrast theme.
    pub is_high_contrast: bool,
}
