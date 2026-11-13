//! What the banner of an operation that stopped says and offers, the message when one stops on conflicts, and where a
//! file in conflict opens, from states made by hand.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::CommitId;
use model::conflicts::RuleCounts;
use model::rebase::{RebaseStop, StopKind};
use model::state::{ConflictKind, ConflictedFile, Operation};

use super::{BannerState, StoppedState, open_target};
use crate::diff::DiffVersion;
use crate::rebase::EditStopState;

/// A merge stopped with 3 files in conflict, the conflicts not counted, both settings off.
fn merge_stopped() -> BannerState<'static> {
    BannerState {
        operation: Some(Operation::Merge),
        conflicted: 3,
        stop: None,
        counts: None,
        auto_resolve: false,
        ai: false,
        is_running: false,
    }
}

/// The words of the buttons of a banner, from the left.
fn labels(banner: &BannerState<'_>) -> Vec<String> {
    banner.buttons().into_iter().map(|button| button.label).collect()
}

/// The counts of 3 conflicts of which some need no choice.
fn three_of_which(resolvable: usize) -> RuleCounts {
    RuleCounts {
        conflicts: 3,
        resolvable,
    }
}

#[test]
fn conflicts_banner__with_auto_resolve_off_no_resolve_button_is_drawn() {
    let banner = BannerState {
        counts: Some(three_of_which(3)),
        ..merge_stopped()
    };
    assert_eq!(labels(&banner), ["Continue", "Abort"]);
}

#[test]
fn conflicts_banner__the_resolve_button_says_how_many_conflicts_need_no_choice() {
    let banner = |counts| BannerState {
        counts,
        auto_resolve: true,
        ..merge_stopped()
    };
    assert_eq!(
        labels(&banner(Some(three_of_which(2)))).first().unwrap(),
        "Resolve 2 of 3 automatically"
    );
    assert_eq!(
        labels(&banner(Some(three_of_which(3)))).first().unwrap(),
        "Resolve all 3 automatically"
    );
    let one = Some(RuleCounts {
        conflicts: 1,
        resolvable: 1,
    });
    assert_eq!(labels(&banner(one)).first().unwrap(), "Resolve it automatically");
    assert_eq!(labels(&banner(Some(three_of_which(0)))), ["Continue", "Abort"]);
}

#[test]
fn conflicts_banner__resolve_with_ai_is_not_drawn_when_the_rules_resolve_every_conflict() {
    let banner = |resolvable| BannerState {
        counts: Some(three_of_which(resolvable)),
        auto_resolve: true,
        ai: true,
        ..merge_stopped()
    };
    assert_eq!(labels(&banner(3)), ["Resolve all 3 automatically", "Continue", "Abort"]);
    assert_eq!(
        labels(&banner(2)),
        ["Resolve 2 of 3 automatically", "Resolve with AI", "Continue", "Abort"]
    );
}

#[test]
fn conflicts_banner__at_a_stop_to_edit_a_commit_it_shows_the_edit_text_with_continue_and_abort_and_no_skip() {
    let stop = RebaseStop {
        kind: StopKind::Edit,
        commit: CommitId::parse(&"c".repeat(40)).unwrap(),
        subject: String::from("Add the login page"),
    };
    let banner = BannerState {
        operation: Some(Operation::Rebase),
        conflicted: 0,
        stop: Some(EditStopState {
            stop: &stop,
            has_conflicts: false,
        }),
        ..merge_stopped()
    };
    assert_eq!(
        banner.says(),
        "Stopped at \"Add the login page\" to edit it. Amend it or commit more, then Continue."
    );
    assert_eq!(labels(&banner), ["Continue", "Abort"]);
    assert!(banner.buttons().iter().all(|button| button.is_enabled));
}

#[test]
fn conflicts_banner__every_button_is_disabled_while_gitardo_runs_something() {
    let banner = BannerState {
        operation: Some(Operation::CherryPick),
        conflicted: 0,
        is_running: true,
        ..merge_stopped()
    };
    assert_eq!(labels(&banner), ["Continue", "Skip", "Abort"]);
    assert!(banner.buttons().iter().all(|button| !button.is_enabled));
}

/// The message of a merge stopped with 3 files in conflict, the conflicts counted.
fn stopped(counts: Option<RuleCounts>, (auto_resolve, ai): (bool, bool)) -> StoppedState<'static> {
    StoppedState {
        title: "Merge feature into main",
        conflicted: 3,
        operation: Some(Operation::Merge),
        counts,
        auto_resolve,
        ai,
    }
}

/// The words of the buttons of a message, from the left.
fn stopped_labels(state: &StoppedState<'_>) -> Vec<&'static str> {
    state.buttons().into_iter().map(|(label, _)| label).collect()
}

#[test]
fn conflicts_stopped__with_2_of_3_resolvable_it_says_so_and_offers_resolve_automatically() {
    let state = stopped(Some(three_of_which(2)), (true, false));
    assert!(
        state
            .says()
            .ends_with("Gitardo can resolve 2 of the 3 conflicts automatically."),
        "{}",
        state.says()
    );
    assert_eq!(stopped_labels(&state), ["Resolve automatically", "Show log"]);
}

#[test]
fn conflicts_stopped__with_both_settings_off_the_only_button_is_show_log() {
    let state = stopped(Some(three_of_which(2)), (false, false));
    assert_eq!(
        state.says(),
        "Merge feature into main stopped: 3 files have conflicts. Resolve them, then continue or abort the merge."
    );
    assert_eq!(stopped_labels(&state), ["Show log"]);
}

/// A file in conflict.
fn conflicted(path: &str, kind: ConflictKind) -> ConflictedFile {
    ConflictedFile {
        path: path.to_owned(),
        kind,
    }
}

#[test]
fn conflicts_open_file__a_file_deleted_by_the_branch_merged_opens_its_base_beside_the_working_tree() {
    let target = open_target(
        &conflicted("src/app.rs", ConflictKind::DeletedByThem),
        Some(Operation::Merge),
    )
    .unwrap();
    assert_eq!(
        target.title.as_deref(),
        Some("app.rs (deleted by the branch being merged, changed by the branch checked out)")
    );
    assert_eq!(
        (target.left, target.right),
        (DiffVersion::Base, DiffVersion::WorkingTree)
    );
}

#[test]
fn conflicts_open_file__a_file_both_sides_deleted_opens_nothing() {
    assert_eq!(
        open_target(
            &conflicted("src/app.rs", ConflictKind::BothDeleted),
            Some(Operation::Merge)
        ),
        None
    );
}
