//! The words of patches: copied, saved and applied (`patch.`).

/// **Copy as patch** in the menu of a commit (`patch.copy`).
pub const COPY_AS_PATCH: &str = "Copy as patch";
/// **Save as patch…** in the menu of a commit (`patch.save`).
pub const SAVE_AS_PATCH: &str = "Save as patch\u{2026}";
/// The tooltip of **Copy as patch** in the bar of a selection of several commits.
pub const COPY_SELECTION_TOOLTIP: &str =
    "Copies them as one patch, oldest first, with their authors and messages, for git am to make them again";
/// **Apply patch…** in the **…** menu of the header (`patch.apply`).
pub const APPLY_PATCH: &str = "Apply patch\u{2026}";
