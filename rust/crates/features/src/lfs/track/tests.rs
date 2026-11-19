use model::contract::{Answer, Answers, FollowUp, Outcome, Query, QuestionKind, Step, Target};

use super::{HOW, PATTERN, TO_STORE, track};
use crate::lfs::testing::{lfs_state, without_lfs};
use crate::testing::{context, lines, plan, question};

/// Answers of how the files are named, and of the files to store read before the plan.
fn answered(how: &str, to_store: &[&str]) -> Answers {
    Answers::new().with(HOW, Answer::Choice(how.to_owned())).with(
        TO_STORE,
        Answer::Names(to_store.iter().map(|path| (*path).to_owned()).collect()),
    )
}

/// The repository set up, with no pattern, and the operation invoked on a file.
fn on(path: &str) -> model::contract::Context {
    context(lfs_state(true, &[]), Target::File(path.to_owned()))
}

#[test]
fn lfs_track__all_psd_files_on_art_logo_psd_runs_git_lfs_track_star_psd_at_the_top() {
    let asked = question(track(&on("art/logo.psd"), &Answers::new()));
    let QuestionKind::Pick { choices } = &asked.kind else {
        panic!("it asks how the files are named");
    };
    let labels: Vec<&str> = choices.iter().map(|choice| choice.label.as_str()).collect();
    assert_eq!(
        labels,
        ["All .psd files", "This file", "Everything in art", "A pattern…"]
    );
    assert_eq!(asked.default, Some(Answer::Choice(String::from("extension"))));
    let picked = Answers::new().with(HOW, Answer::Choice(String::from("extension")));
    assert!(matches!(
        track(&on("art/logo.psd"), &picked),
        Outcome::Read(TO_STORE, Query::LfsToStore { pathspecs }) if pathspecs == [":(glob)**/*.psd"]
    ));
    let ran = plan(track(&on("art/logo.psd"), &answered("extension", &[])));
    assert_eq!(lines(&ran), ["git lfs track *.psd"]);
    assert_eq!(
        ran.steps[0],
        Step::git(crate::testing::state().folder, ["lfs", "track", "*.psd"])
    );
    // Everything in the folder, and a pattern typed.
    let folder = plan(track(&on("art/logo.psd"), &answered("folder", &[])));
    assert_eq!(lines(&folder), ["git lfs track art/**"]);
    let typed = answered("typed", &[]).with(PATTERN, Answer::Text(String::from(" *.tif ")));
    assert_eq!(
        lines(&plan(track(&on("art/logo.psd"), &typed))),
        ["git lfs track *.tif"]
    );
}

#[test]
fn lfs_track__this_file_on_a_name_with_brackets_runs_git_lfs_track_filename() {
    let ran = plan(track(&on("art/logo [1].psd"), &answered("file", &[])));
    assert_eq!(lines(&ran), ["git lfs track --filename \"art/logo [1].psd\""]);
    let Outcome::Read(_, Query::LfsToStore { pathspecs }) = track(
        &on("art/logo [1].psd"),
        &Answers::new().with(HOW, Answer::Choice(String::from("file"))),
    ) else {
        panic!("it reads the files the path takes");
    };
    assert_eq!(pathspecs, [":(literal)art/logo [1].psd"]);
}

#[test]
fn lfs_track__with_logo_psd_committed_before_the_message_counts_1_file_and_offers_to_store_it() {
    let ran = plan(track(&on("logo.psd"), &answered("extension", &["logo.psd"])));
    let said = ran.after.iter().find_map(|follow_up| match follow_up {
        FollowUp::Say(text) => Some(text.as_str().to_owned()),
        _ => None,
    });
    assert_eq!(
        said.as_deref(),
        Some(
            "Files that *.psd takes are stored with Git LFS from their next commit. Commit .gitattributes with them. \
             1 file committed before is not stored with Git LFS yet."
        )
    );
    let Some(FollowUp::OfferRun(offered)) = ran.after.last() else {
        panic!("it offers to store the file");
    };
    assert_eq!(offered.feature.0, "lfs.store-committed");
    assert_eq!(offered.label.as_str(), "Store them with Git LFS in the next commit");
    assert_eq!(
        offered.answers.names(crate::lfs::store_committed::FILES),
        Some(&[String::from("logo.psd")][..])
    );
    // From the page, with no file, the pattern is typed at once.
    let page = context(lfs_state(true, &[]), Target::Head);
    assert!(matches!(
        question(track(&page, &Answers::new())).kind,
        QuestionKind::Text { .. }
    ));
}

#[test]
fn lfs_track__not_set_up_it_refuses_with_set_up_and_nothing_runs() {
    let not_set_up = context(lfs_state(false, &[]), Target::File(String::from("logo.psd")));
    let Outcome::Refuse(refusal) = track(&not_set_up, &answered("extension", &[])) else {
        panic!("it refuses");
    };
    assert_eq!(refusal.message.as_str(), "Git LFS is not set up in this repository.");
    let (feature, words) = refusal.offer.expect("Set up is offered");
    assert_eq!((feature.0, words.as_str()), ("lfs.set-up", "Set up"));
    let without = context(without_lfs(), Target::File(String::from("logo.psd")));
    assert!(matches!(
        track(&without, &answered("extension", &[])),
        Outcome::Refuse(_)
    ));
}
