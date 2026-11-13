//! Every operation of Gitardo Desktop, as a function that asks questions and returns a plan.
//!
//! Each operation is pure (rule R2.2): it is given the [`Context`] and the [`Answers`] so far, and says what it needs
//! next or what it has decided to do. Only the engine runs what it decides.

#![forbid(unsafe_code)]

pub mod bisect;
pub mod blame;
pub mod branch;
pub mod changes;
pub mod cherry_pick;
pub mod commit;
pub mod commit_page;
pub mod commits;
pub mod compare;
pub mod conflicts;
pub mod diff;
pub mod file_history;
pub mod file_view;
pub mod fixup;
pub mod github;
mod history;
pub mod identity;
pub mod merge;
pub mod merge_view;
pub mod message;
pub mod patch;
pub mod rebase;
pub mod reflog;
pub mod remote;
pub mod reset;
pub mod revert;
pub mod review;
pub mod search;
pub mod selection;
pub mod split;
pub mod squash;
pub mod stash;
pub mod submodule;
pub mod sync;
pub mod tag;
pub mod undo;
pub mod worktree;
pub mod worktree_setup;

#[cfg(test)]
mod testing;

use model::contract::{FeatureId, Operation};

/// The operations of the catalog that are built, by identifier.
pub const OPERATIONS: [(FeatureId, Operation); 151] = [
    (FeatureId("branch.checkout"), branch::checkout),
    (FeatureId("branch.checkout-detached"), branch::checkout_detached),
    (FeatureId("branch.checkout-remote"), branch::checkout_remote),
    (
        FeatureId("branch.checkout-remote-and-rebase"),
        branch::checkout_remote_and_rebase,
    ),
    (FeatureId("branch.clean-up"), branch::clean_up),
    (FeatureId("branch.create"), branch::create),
    (FeatureId("branch.delete"), branch::delete),
    (FeatureId("branch.fast-forward"), branch::fast_forward),
    (FeatureId("branch.rename"), branch::rename),
    (FeatureId("changes.discard"), changes::discard),
    (FeatureId("changes.discard-all"), changes::discard_all),
    (FeatureId("changes.save"), changes::save),
    (FeatureId("changes.stage"), changes::stage),
    (FeatureId("changes.stage-all"), changes::stage_all),
    (FeatureId("changes.unstage"), changes::unstage),
    (FeatureId("changes.unstage-all"), changes::unstage_all),
    (FeatureId("commit-page.open"), commit_page::open),
    (FeatureId("commit.amend"), commit::amend),
    (FeatureId("commit.create"), commit::create),
    (FeatureId("commits.copy-hash"), commits::copy_hash),
    (FeatureId("commits.copy-message"), commits::copy_message),
    (FeatureId("identity.switch"), identity::switch),
    (FeatureId("identity.use-global"), identity::use_global),
    (FeatureId("search.go-to-commit"), search::go_to_commit),
    (FeatureId("sync.fetch"), sync::fetch),
    (FeatureId("tag.checkout"), tag::checkout),
    (FeatureId("tag.create"), tag::create),
    (FeatureId("tag.delete"), tag::delete),
    (FeatureId("tag.push"), tag::push),
    (FeatureId("undo.last"), undo::last),
    // Remotes, sync and stashes.
    (FeatureId("sync.pull"), sync::pull),
    (FeatureId("sync.pull-with"), sync::pull_with),
    (FeatureId("sync.push"), sync::push),
    (FeatureId("sync.push-force"), sync::push_force),
    (FeatureId("sync.push-force-with-lease"), sync::push_force_with_lease),
    (FeatureId("sync.sync"), sync::sync),
    (FeatureId("remote.add"), remote::add),
    (FeatureId("remote.delete-branch"), remote::delete_branch),
    (FeatureId("remote.fetch"), remote::fetch),
    (FeatureId("remote.manage"), remote::manage),
    (FeatureId("remote.remove"), remote::remove),
    (FeatureId("remote.rename"), remote::rename),
    (FeatureId("remote.set-url"), remote::set_url),
    (FeatureId("stash.apply"), stash::apply),
    (FeatureId("stash.branch"), stash::branch),
    (FeatureId("stash.drop"), stash::drop),
    (FeatureId("stash.pop"), stash::pop),
    (FeatureId("stash.push"), stash::push),
    (FeatureId("stash.push-file"), stash::push_file),
    (FeatureId("stash.push-staged"), stash::push_staged),
    (FeatureId("stash.undo-drop"), stash::undo_drop),
    // The Diff page.
    (FeatureId("diff.discard-file"), diff::discard_file),
    (FeatureId("diff.revert-change"), diff::revert_change),
    (FeatureId("diff.stage-change"), diff::stage_change),
    (FeatureId("diff.stage-file"), diff::stage_file),
    (FeatureId("diff.unstage-change"), diff::unstage_change),
    (FeatureId("diff.unstage-file"), diff::unstage_file),
    (FeatureId("file-view.open-external"), file_view::open_external),
    (FeatureId("file-view.save"), file_view::save),
    // Commits brought, taken back and reworded, and several selected at once.
    (FeatureId("cherry-pick.commit"), cherry_pick::commit),
    (FeatureId("cherry-pick.skip-empty"), cherry_pick::skip_empty),
    (FeatureId("merge.choose-way"), merge::choose_way),
    (FeatureId("merge.into-current"), merge::into_current),
    (FeatureId("merge.undo-squash"), merge::undo_squash),
    (FeatureId("message.edit"), message::edit),
    (FeatureId("reset.to-commit"), reset::to_commit),
    (FeatureId("revert.commit"), revert::commit),
    (FeatureId("selection.cherry-pick"), selection::cherry_pick),
    // Rebase and rewriting.
    (FeatureId("rebase.onto"), rebase::onto),
    (FeatureId("rebase.branch-onto"), rebase::branch_onto),
    (FeatureId("rebase.interactive"), rebase::interactive),
    (FeatureId("rebase.edit-stop"), rebase::continue_edit),
    (FeatureId("squash.commits"), squash::commits),
    (FeatureId("squash.into-previous"), squash::into_previous),
    (FeatureId("split.commit"), split::commit),
    (FeatureId("split.continue"), split::continue_split),
    (FeatureId("fixup.commit"), fixup::commit),
    // Bisect.
    (FeatureId("bisect.answer"), bisect::answer),
    (FeatureId("bisect.end"), bisect::end),
    (FeatureId("bisect.mark"), bisect::mark),
    (FeatureId("bisect.start"), bisect::start),
    // Patches.
    (FeatureId("patch.apply"), patch::apply),
    (FeatureId("patch.copy"), patch::copy),
    (FeatureId("patch.save"), patch::save),
    // Comparisons.
    (FeatureId("compare.change-side"), compare::change_side),
    (FeatureId("compare.choose"), compare::choose),
    (FeatureId("compare.copy-patch"), compare::copy_patch),
    (FeatureId("compare.path-with"), compare::path_with),
    (FeatureId("compare.selected-before"), compare::selected_before),
    (FeatureId("compare.take-file"), compare::take_file),
    (FeatureId("compare.with"), compare::with),
    (FeatureId("compare.with-current"), compare::with_current),
    // The reflog.
    (FeatureId("reflog.browse"), reflog::browse),
    // Conflicts.
    (FeatureId("conflicts.abort-operation"), conflicts::abort_operation),
    (FeatureId("conflicts.continue-operation"), conflicts::continue_operation),
    (FeatureId("conflicts.mark-resolved"), conflicts::mark_resolved),
    (
        FeatureId("conflicts.resolve-automatically"),
        conflicts::resolve_automatically,
    ),
    (FeatureId("conflicts.restore"), conflicts::restore),
    (FeatureId("conflicts.skip-step"), conflicts::skip_step),
    (FeatureId("conflicts.take-side"), conflicts::take_side),
    // The Merge page.
    (FeatureId("merge-view.mark-resolved"), merge_view::mark_resolved),
    // The review of the conflicts.
    (FeatureId("review.write"), review::write),
    // Submodules.
    (FeatureId("submodule.add"), submodule::add),
    (FeatureId("submodule.copy-path"), submodule::copy_path),
    (FeatureId("submodule.copy-url"), submodule::copy_url),
    (FeatureId("submodule.deinitialize"), submodule::deinitialize),
    (FeatureId("submodule.initialize"), submodule::initialize),
    (FeatureId("submodule.open-in-new-window"), submodule::open_in_new_window),
    (FeatureId("submodule.open-in-terminal"), submodule::open_in_terminal),
    (FeatureId("submodule.remove"), submodule::remove),
    (FeatureId("submodule.reveal"), submodule::reveal),
    (FeatureId("submodule.show-in-gitardo"), submodule::show_in_gitardo),
    (FeatureId("submodule.stage-recorded"), submodule::stage_recorded),
    (FeatureId("submodule.sync-urls"), submodule::sync_urls),
    (FeatureId("submodule.update-to-latest"), submodule::update_to_latest),
    (FeatureId("submodule.update-to-recorded"), submodule::update_to_recorded),
    // Worktrees.
    (FeatureId("worktree.copy-path"), worktree::copy_path),
    (FeatureId("worktree.create"), worktree::create),
    (FeatureId("worktree.create-at-commit"), worktree::create_at_commit),
    (FeatureId("worktree.create-from-branch"), worktree::create_from_branch),
    (FeatureId("worktree.open"), worktree::open),
    (FeatureId("worktree.open-in-editor"), worktree::open_in_editor),
    (FeatureId("worktree.open-in-new-window"), worktree::open_in_new_window),
    (FeatureId("worktree.open-in-terminal"), worktree::open_in_terminal),
    (FeatureId("worktree.open-in-worktree"), worktree::open_in_worktree),
    (FeatureId("worktree.prune"), worktree::prune),
    (FeatureId("worktree.remove"), worktree::remove),
    (FeatureId("worktree.rename"), worktree::rename),
    (FeatureId("worktree.reveal"), worktree::reveal),
    (FeatureId("worktree.show-in-gitardo"), worktree::show_in_gitardo),
    (FeatureId("worktree.show-in-graph"), worktree::show_in_graph),
    (FeatureId("worktree.switch"), worktree::switch),
    // What new worktrees get.
    (
        FeatureId("worktree-setup.bring-into-other-worktrees"),
        worktree_setup::bring_into_other_worktrees,
    ),
    (
        FeatureId("worktree-setup.bring-into-worktree"),
        worktree_setup::bring_into_worktree,
    ),
    (
        FeatureId("worktree-setup.copy-into-new-worktrees"),
        worktree_setup::copy_into_new_worktrees,
    ),
    (
        FeatureId("worktree-setup.link-into-new-worktrees"),
        worktree_setup::link_into_new_worktrees,
    ),
    (FeatureId("worktree-setup.remove-from"), worktree_setup::remove_from),
    (FeatureId("worktree-setup.save"), worktree_setup::save),
    (FeatureId("worktree-setup.stop-copying"), worktree_setup::stop_copying),
    (FeatureId("worktree-setup.stop-linking"), worktree_setup::stop_linking),
    // The history of a file, of lines or of a folder, and the blame.
    (FeatureId("file-history.file"), file_history::file),
    (FeatureId("file-history.folder"), file_history::folder),
    (FeatureId("file-history.lines"), file_history::lines),
    (FeatureId("blame.every-file"), blame::every_file),
    (FeatureId("blame.ignore-whitespace"), blame::ignore_whitespace),
    (FeatureId("blame.open-line-commit"), blame::open_line_commit),
    (FeatureId("blame.previous"), blame::previous),
    (FeatureId("blame.reveal-line-commit"), blame::reveal_line_commit),
    (FeatureId("blame.toggle-current-line"), blame::toggle_current_line),
    (FeatureId("blame.toggle-file"), blame::toggle_file),
    // GitHub.
    (FeatureId("github.load-pull-requests"), github::load_pull_requests),
];

/// The operation of an identifier, when it is built.
#[must_use]
pub fn operation(identifier: &str) -> Option<(FeatureId, Operation)> {
    OPERATIONS.iter().find(|(feature, _)| feature.0 == identifier).copied()
}
