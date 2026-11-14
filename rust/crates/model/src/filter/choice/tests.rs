//! The choice of the branches shown, changed by presets, ticks, kinds, and the entries of the menus.

use super::super::testing::references;
use super::super::{BranchesShown, Kinds, Preset, ReferenceKind, label, log_arguments};
use super::{hide, kinds, kinds_shown, preset, show_all, show_only, show_too, tick};
use crate::graph::testing::id;
use crate::refs::References;

fn ticked(names: &[&str]) -> BranchesShown {
    BranchesShown::References(names.iter().map(|name| (*name).to_owned()).collect())
}

fn repository() -> References {
    references(&["main", "feature", "fix"], &["origin/main"], &["v1.0"])
}

#[test]
fn filter_tick__from_all_ticking_main_shows_main_alone() {
    assert_eq!(
        tick(None, "refs/heads/main", &repository()),
        Some(ticked(&["refs/heads/main"]))
    );
}

#[test]
fn filter_tick__from_local_unticking_feature_shows_main_alone_ticked_one_by_one() {
    let repository = references(&["main", "feature"], &["origin/main"], &[]);
    let local = preset(Preset::Local);
    assert_eq!(
        tick(local.as_ref(), "refs/heads/feature", &repository),
        Some(ticked(&["refs/heads/main"]))
    );
}

#[test]
fn filter_tick__from_upstream_on_main_ticking_v1_0_shows_main_origin_main_and_v1_0() {
    assert_eq!(
        tick(Some(&BranchesShown::Upstream), "refs/tags/v1.0", &repository()),
        Some(ticked(&[
            "refs/heads/main",
            "refs/remotes/origin/main",
            "refs/tags/v1.0"
        ]))
    );
}

#[test]
fn filter_tick__unticking_the_only_reference_ticked_shows_everything() {
    let shown = ticked(&["refs/heads/main"]);
    assert_eq!(tick(Some(&shown), "refs/heads/main", &repository()), None);
}

#[test]
fn filter_tick__a_branch_and_a_tag_of_the_same_name_are_two() {
    let repository = references(&["v1.0"], &[], &["v1.0"]);
    let shown = tick(None, "refs/heads/v1.0", &repository);
    let both = tick(shown.as_ref(), "refs/tags/v1.0", &repository);
    assert_eq!(both, Some(ticked(&["refs/heads/v1.0", "refs/tags/v1.0"])));
}

#[test]
fn filter_kinds__from_all_unchecking_show_tags_shows_the_branches_and_the_button_says_branches() {
    let shown = kinds(None, ReferenceKind::Tag);
    assert_eq!(
        shown,
        Some(BranchesShown::Kinds(Kinds {
            local: true,
            remote: true,
            tags: false
        }))
    );
    assert_eq!(label(shown.as_ref()), "Branches");
}

#[test]
fn filter_kinds__unchecking_show_remote_branches_then_is_the_same_as_local() {
    let branches = kinds(None, ReferenceKind::Tag);
    assert_eq!(kinds(branches.as_ref(), ReferenceKind::Remote), preset(Preset::Local));
}

#[test]
fn filter_kinds__with_only_the_tags_shown_show_tags_cannot_be_unchecked() {
    let tags = Some(BranchesShown::Kinds(Kinds::only(ReferenceKind::Tag)));
    assert_eq!(kinds(tags.as_ref(), ReferenceKind::Tag), tags);
    assert_eq!(
        kinds(tags.as_ref(), ReferenceKind::Local),
        Some(BranchesShown::Kinds(Kinds {
            local: true,
            remote: false,
            tags: true
        }))
    );
}

#[test]
fn filter_kinds__with_main_ticked_alone_the_kinds_are_chosen_in_the_filter() {
    let shown = ticked(&["refs/heads/main"]);
    assert_eq!(kinds_shown(Some(&shown)), None);
    assert_eq!(kinds(Some(&shown), ReferenceKind::Tag), Some(shown));
    assert_eq!(kinds_shown(Some(&BranchesShown::Current)), None);
    assert_eq!(kinds_shown(None), Some(Kinds::ALL));
}

#[test]
fn filter_show_too__narrowed_to_main_feature_too_shows_both_and_the_button_says_main_and_1() {
    let shown = show_too(&ticked(&["refs/heads/main"]), "refs/heads/feature", &repository());
    assert_eq!(shown, ticked(&["refs/heads/main", "refs/heads/feature"]));
    assert_eq!(label(Some(&shown)), "main +1");
}

#[test]
fn filter_show_too__from_current_on_main_it_shows_main_and_feature_no_longer_following() {
    let shown = show_too(&BranchesShown::Current, "refs/heads/feature", &repository());
    assert_eq!(shown, ticked(&["refs/heads/main", "refs/heads/feature"]));
}

#[test]
fn filter_hide__narrowed_to_main_and_feature_hiding_feature_shows_main_alone() {
    let shown = ticked(&["refs/heads/main", "refs/heads/feature"]);
    assert_eq!(
        hide(&shown, "refs/heads/feature", &repository()),
        ticked(&["refs/heads/main"])
    );
}

#[test]
fn filter_hide__narrowed_to_local_hiding_fix_shows_every_other_local_branch_ticked_one_by_one() {
    let local = preset(Preset::Local).unwrap_or(BranchesShown::Current);
    assert_eq!(
        hide(&local, "refs/heads/fix", &repository()),
        ticked(&["refs/heads/main", "refs/heads/feature"])
    );
}

#[test]
fn filter_hide__the_only_reference_shown_stays() {
    let shown = ticked(&["refs/heads/main"]);
    assert_eq!(hide(&shown, "refs/heads/main", &repository()), shown);
}

#[test]
fn filter_show_only__the_reference_alone_whatever_was_shown_before() {
    assert_eq!(show_only("refs/tags/v1.0"), ticked(&["refs/tags/v1.0"]));
    assert_eq!(show_all(), None);
}

#[test]
fn filter_preset__upstream_on_main_tracking_origin_main_lists_both_and_no_other() {
    let head = id("A");
    let upstream = Some("refs/remotes/origin/main");
    let arguments = log_arguments(preset(Preset::Upstream).as_ref(), Some(&head), upstream);
    assert_eq!(arguments, ["--date-order", "HEAD", "refs/remotes/origin/main"]);
}

#[test]
fn filter_preset__upstream_with_no_upstream_or_detached_lists_what_is_checked_out_alone() {
    let head = id("A");
    let arguments = log_arguments(preset(Preset::Upstream).as_ref(), Some(&head), None);
    assert_eq!(arguments, ["--date-order", "HEAD"]);
}

#[test]
fn filter_preset__local_and_remotes_list_what_is_checked_out_too() {
    let head = id("A");
    let local = log_arguments(preset(Preset::Local).as_ref(), Some(&head), None);
    assert_eq!(local, ["--date-order", "--branches", "HEAD"]);
    let remotes = log_arguments(preset(Preset::Remotes).as_ref(), Some(&head), None);
    assert_eq!(remotes, ["--date-order", "--remotes", "HEAD"]);
    assert_eq!(preset(Preset::All), None);
}
