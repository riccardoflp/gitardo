//! A patch applied with `git am` as the window shows it while it is under way (`docs/features/14-patches.md`).

use ui::Ui;

use crate::conflicts::BannerState;
use crate::intent::Intent;

/// The banner of a patch whose commit did not fit and that waits, as a cherry-pick does: **Patch (git am) in
/// progress**, with **Continue** (`git -c core.editor=true am --continue`), **Skip** (`git am --skip`) and **Abort**
/// (`git am --abort`), the banner of every operation that stopped (`conflicts.banner`). The sides of its files in
/// conflict are named **The branch checked out** and **The patch being applied** where the conflicts are read
/// (`model::conflicts`).
///
/// Feature: patch.am-stopped
pub fn am_stopped(state: &BannerState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    crate::conflicts::operation_banner(state, ui)
}
