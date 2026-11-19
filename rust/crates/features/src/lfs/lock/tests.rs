use model::contract::{Answers, Outcome, Target};

use super::lock;
use crate::lfs::testing::{lfs_state, without_lfs};
use crate::testing::{context, lines, plan};

#[test]
fn lfs_lock__lock_on_art_logo_psd_runs_git_lfs_lock_json_art_logo_psd() {
    let on_file = context(
        lfs_state(true, &[("*.psd", ".gitattributes")]),
        Target::File(String::from("art/logo.psd")),
    );
    let ran = plan(lock(&on_file, &Answers::new()));
    assert_eq!(lines(&ran), ["git lfs lock --json art/logo.psd"]);
    assert_eq!(ran.title.as_str(), "Lock art/logo.psd");
    let without = context(without_lfs(), Target::File(String::from("art/logo.psd")));
    assert!(matches!(lock(&without, &Answers::new()), Outcome::Refuse(_)));
    let on_head = context(lfs_state(true, &[]), Target::Head);
    assert!(matches!(lock(&on_head, &Answers::new()), Outcome::Refuse(_)));
}
