//! When a plan is shown to the user before it runs (`docs/desktop/04-feature-contract.md`, "Danger and confirmation").

use model::contract::{CommandPreview, Danger};

/// Whether a plan of this danger is shown and confirmed before it runs, with `commandPreview` set so.
///
/// | `commandPreview` | None | Confirm | Destructive |
/// | --- | --- | --- | --- |
/// | `off` | runs | asks | asks, with the warning |
/// | `all` | asks | asks | asks, with the warning |
///
/// A plan the user agreed to in the feature's own question (`Agreed`) runs without being shown again, either way.
#[must_use]
pub fn needs_confirmation(danger: &Danger, preview: CommandPreview) -> bool {
    match (danger, preview) {
        (Danger::None, CommandPreview::Off) | (Danger::Agreed, _) => false,
        (Danger::None, CommandPreview::All) | (Danger::Confirm(_) | Danger::Destructive(_), _) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::needs_confirmation;
    use model::contract::{CommandPreview, Danger, Text};

    #[test]
    fn follows_the_table_of_the_contract() {
        let reason = Text::from("It moves a branch.");
        let cases = [
            (Danger::None, CommandPreview::Off, false),
            (Danger::None, CommandPreview::All, true),
            (Danger::Agreed, CommandPreview::Off, false),
            (Danger::Agreed, CommandPreview::All, false),
            (Danger::Confirm(reason.clone()), CommandPreview::Off, true),
            (Danger::Confirm(reason.clone()), CommandPreview::All, true),
            (Danger::Destructive(reason.clone()), CommandPreview::Off, true),
            (Danger::Destructive(reason), CommandPreview::All, true),
        ];
        for (danger, preview, expected) in cases {
            assert_eq!(needs_confirmation(&danger, preview), expected, "{danger:?} {preview:?}");
        }
    }
}
