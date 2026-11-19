use model::contract::{Answer, Answers, Outcome, Query, Target};

use super::{FILES, store_committed};
use crate::lfs::testing::{lfs_state, without_lfs};
use crate::testing::{context, lines, plan};

/// The answer of the files to store.
fn files(paths: &[&str]) -> Answers {
    Answers::new().with(
        FILES,
        Answer::Names(paths.iter().map(|path| (*path).to_owned()).collect()),
    )
}

/// A repository that stores `*.psd` with Git LFS.
fn psd() -> model::contract::Context {
    context(lfs_state(true, &[("*.psd", ".gitattributes")]), Target::Head)
}

#[test]
fn lfs_store_committed__logo_psd_committed_as_a_plain_file_is_added_again_with_renormalize() {
    let ran = plan(store_committed(&psd(), &files(&["logo.psd"])));
    assert_eq!(lines(&ran), ["git add --renormalize -- logo.psd"]);
    assert_eq!(ran.title.as_str(), "Store logo.psd with Git LFS in the next commit");
    let two = plan(store_committed(&psd(), &files(&["logo.psd", "art/logo [1].psd"])));
    assert_eq!(
        lines(&two),
        ["git add --renormalize -- logo.psd \":(literal)art/logo [1].psd\""]
    );
}

#[test]
fn lfs_store_committed__a_file_stored_with_git_lfs_already_is_not_named() {
    // The files to store are those of every pattern that Git LFS does not store yet, read before the plan.
    assert!(matches!(
        store_committed(&psd(), &Answers::new()),
        Outcome::Read(FILES, Query::LfsToStore { pathspecs }) if pathspecs == [":(attr:filter=lfs)"]
    ));
    assert!(matches!(store_committed(&psd(), &files(&[])), Outcome::Refuse(_)));
}

#[test]
fn lfs_store_committed__the_commits_before_keep_logo_psd_as_they_had_it() {
    // Only the index is changed: the plan is one `git add`, and nothing rewrites a commit.
    let ran = plan(store_committed(&psd(), &files(&["logo.psd"])));
    assert!(lines(&ran).iter().all(|line| line.starts_with("git add ")));
    let without = context(without_lfs(), Target::Head);
    assert!(matches!(
        store_committed(&without, &files(&["logo.psd"])),
        Outcome::Refuse(_)
    ));
}
