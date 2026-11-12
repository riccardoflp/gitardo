//! Patches: what applying one would do.

/// What applying a patch with `--3way` would do, as `git apply --check` says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchCheck {
    /// It applies, leaving these files in conflict; none when it applies cleanly.
    Applies {
        /// The files it would leave in conflict.
        conflicts: Vec<String>,
    },
    /// It does not apply at all.
    Refused {
        /// Why, in git's words, each reason once.
        reasons: Vec<String>,
    },
}
