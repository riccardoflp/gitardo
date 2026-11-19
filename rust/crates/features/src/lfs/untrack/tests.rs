use model::contract::{Answer, Answers, Outcome, QuestionKind, Target};

use super::{PATTERN, untrack};
use crate::lfs::testing::{lfs_state, without_lfs};
use crate::testing::{context, lines, plan, question};

/// The pattern given by the menu of a pattern.
fn pattern(pattern: &str) -> Answers {
    Answers::new().with(PATTERN, Answer::Text(pattern.to_owned()))
}

#[test]
fn lfs_untrack__on_star_psd_it_runs_git_lfs_untrack_star_psd() {
    let repository = context(
        lfs_state(true, &[("*.psd", ".gitattributes"), ("*.png", "assets/.gitattributes")]),
        Target::Head,
    );
    let ran = plan(untrack(&repository, &pattern("*.psd")));
    assert_eq!(lines(&ran), ["git lfs untrack *.psd"]);
    assert_eq!(ran.title.as_str(), "Stop storing *.psd with Git LFS");
    // With no pattern given, it asks among those of the .gitattributes at the top.
    let QuestionKind::Pick { choices } = question(untrack(&repository, &Answers::new())).kind else {
        panic!("it asks which pattern");
    };
    assert_eq!(choices.len(), 1);
    let without = context(without_lfs(), Target::Head);
    assert!(matches!(untrack(&without, &pattern("*.psd")), Outcome::Refuse(_)));
}

#[test]
fn lfs_untrack__a_pattern_of_assets_gitattributes_is_refused_and_opened_there() {
    let repository = context(lfs_state(true, &[("*.png", "assets/.gitattributes")]), Target::Head);
    let Outcome::Refuse(refusal) = untrack(&repository, &pattern("*.png")) else {
        panic!("it refuses");
    };
    assert_eq!(
        refusal.message.as_str(),
        "*.png is in assets/.gitattributes: open that file to change it."
    );
}
