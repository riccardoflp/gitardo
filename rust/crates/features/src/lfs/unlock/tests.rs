use model::contract::{Answers, Danger, FollowUp, Target};
use model::lfs::LfsLock;

use super::{UNCOMMITTED, unlock};
use crate::lfs::testing::lfs_state;
use crate::testing::{context, lines, plan};

/// A lock of `art/logo.psd`, of this user or of someone else.
fn locked(is_ours: Option<bool>, owner: &str) -> model::contract::Context {
    let lock = LfsLock {
        id: String::from("42"),
        path: String::from("art/logo.psd"),
        owner: owner.to_owned(),
        locked_at: String::from("2026-11-19T10:00:00Z"),
        is_ours,
    };
    context(
        lfs_state(true, &[("*.psd", ".gitattributes")]),
        Target::LfsLock(Box::new(lock)),
    )
}

#[test]
fn lfs_unlock__unlock_on_a_lock_of_this_user_runs_git_lfs_unlock_art_logo_psd() {
    let ran = plan(unlock(&locked(Some(true), "Test Author"), &Answers::new()));
    assert_eq!(lines(&ran), ["git lfs unlock art/logo.psd"]);
    assert_eq!(ran.danger, Danger::None);
    // A server that did not say whose it is: unlocked as this user's, the server refusing when it is not.
    let unknown = plan(unlock(&locked(None, "designer"), &Answers::new()));
    assert_eq!(lines(&unknown), ["git lfs unlock art/logo.psd"]);
}

#[test]
fn lfs_unlock__unlock_on_a_lock_of_someone_else_asks_with_the_warning_then_runs_force_with_its_id() {
    let ran = plan(unlock(&locked(Some(false), "designer"), &Answers::new()));
    assert_eq!(lines(&ran), ["git lfs unlock --force --id=42"]);
    let Danger::Confirm(warning) = &ran.danger else {
        panic!("it asks first, not {:?}", ran.danger);
    };
    assert_eq!(
        warning.as_str(),
        "The lock is designer's, and they are not told: they may be changing the file."
    );
}

#[test]
fn lfs_unlock__with_the_file_changed_and_not_committed_the_refusal_is_followed_by_unlock_anyway() {
    let ran = plan(unlock(&locked(Some(true), "Test Author"), &Answers::new()));
    let [FollowUp::OfferWhenFailed(offer)] = ran.on_failure.as_slice() else {
        panic!("Unlock anyway is offered when it fails: {:?}", ran.on_failure);
    };
    assert_eq!(offer.when, UNCOMMITTED);
    assert_eq!(offer.plan.title.as_str(), "Unlock anyway");
    assert_eq!(lines(&offer.plan), ["git lfs unlock --force --id=42"]);
}
