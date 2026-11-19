use model::contract::{Answer, Answers, FollowUp, Outcome, Query, Target};

use super::{POINTERS, set_up};
use crate::lfs::testing::{lfs_state, without_lfs};
use crate::testing::{context, lines, plan};

/// The answer of the files left as pointers.
fn pointers(paths: &[&str]) -> Answers {
    Answers::new().with(
        POINTERS,
        Answer::Names(paths.iter().map(|path| (*path).to_owned()).collect()),
    )
}

#[test]
fn lfs_set_up__it_runs_git_lfs_install_local_after_reading_the_files_left_as_pointers() {
    let not_set_up = context(lfs_state(false, &[("*.psd", ".gitattributes")]), Target::Head);
    assert!(matches!(
        set_up(&not_set_up, &Answers::new()),
        Outcome::Read(POINTERS, Query::LfsPointers { version }) if version == "3.7.1"
    ));
    let ran = plan(set_up(&not_set_up, &pointers(&[])));
    assert_eq!(lines(&ran), ["git lfs install --local"]);
    assert_eq!(ran.title.as_str(), "Set up Git LFS in this repository");
    assert!(
        !ran.after
            .iter()
            .any(|follow_up| matches!(follow_up, FollowUp::OfferRun(_)))
    );
    // Set up already, or without Git LFS, it runs nothing.
    let set_up_already = context(lfs_state(true, &[]), Target::Head);
    assert!(matches!(set_up(&set_up_already, &pointers(&[])), Outcome::Refuse(_)));
    assert!(matches!(
        set_up(&context(without_lfs(), Target::Head), &pointers(&[])),
        Outcome::Refuse(_)
    ));
}

#[test]
fn lfs_set_up__after_it_download_all_is_offered_while_a_file_is_a_pointer() {
    let not_set_up = context(lfs_state(false, &[("*.psd", ".gitattributes")]), Target::Head);
    let ran = plan(set_up(&not_set_up, &pointers(&["art/logo.psd"])));
    let offered = ran
        .after
        .iter()
        .find_map(|follow_up| match follow_up {
            FollowUp::OfferRun(offered) => Some(offered),
            _ => None,
        })
        .expect("Download all is offered");
    assert_eq!(offered.feature.0, "lfs.download");
    assert_eq!(offered.label.as_str(), "Download all (1)");
    assert!(matches!(&offered.target, Target::LfsFiles(download) if download.paths.is_empty()));
}
