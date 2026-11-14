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

/// A patch to check or apply: in a file, or its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchInput {
    /// A file that holds it.
    File(std::path::PathBuf),
    /// Its text, given to git on its standard input.
    Text(Vec<u8>),
}
