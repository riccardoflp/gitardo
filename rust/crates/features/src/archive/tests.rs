use std::path::PathBuf;

use model::contract::{Answer, Answers, Outcome, Query, Target};

use super::{EVERY_FILE, FOLDER, FOLDERS, FORMAT, HOLDS, INSIDE, ONE_FOLDER, PATH, TAR_GZ, ZIP, save};
use crate::testing::{commit, context, lines, plan, question, state};

fn on_tag() -> model::contract::Context {
    let mut state = state();
    state.folder = PathBuf::from("/code/app");
    context(state, Target::Tag(String::from("v1.2.0")))
}

fn answered(format: &str, inside: &str) -> Answers {
    Answers::new()
        .with(HOLDS, Answer::Choice(EVERY_FILE.to_owned()))
        .with(FORMAT, Answer::Choice(format.to_owned()))
        .with(INSIDE, Answer::Text(inside.to_owned()))
        .with(PATH, Answer::Path(PathBuf::from("/code/app-v1.2.0.zip")))
}

#[test]
fn archive_save__on_a_tag_it_proposes_its_name_and_archives_its_full_name_with_the_folder_inside() {
    let context = on_tag();
    assert_eq!(question(save(&context, &Answers::new())).key, HOLDS);
    let partial = Answers::new()
        .with(HOLDS, Answer::Choice(EVERY_FILE.to_owned()))
        .with(FORMAT, Answer::Choice(ZIP.to_owned()));
    let inside = question(save(&context, &partial));
    assert_eq!(inside.default, Some(Answer::Text(String::from("app-v1.2.0/"))));
    let asked = question(save(
        &context,
        &partial.with(INSIDE, Answer::Text(String::from("app-v1.2.0/"))),
    ));
    let model::contract::QuestionKind::SaveFile(file) = asked.kind else {
        panic!("the save dialog")
    };
    assert_eq!(file.file_name.as_deref(), Some("app-v1.2.0.zip"));
    assert_eq!(file.folder, PathBuf::from("/code"));
    let plan = plan(save(&context, &answered(ZIP, "app-v1.2.0/")));
    assert_eq!(
        lines(&plan),
        ["git archive --format=zip --prefix=app-v1.2.0/ -o /code/app-v1.2.0.zip refs/tags/v1.2.0"]
    );
}

#[test]
fn archive_save__one_folder_reads_the_folders_of_the_commit_and_adds_it_after_the_dashes() {
    let context = context(state(), Target::Commit(commit('3')));
    let answers = Answers::new().with(HOLDS, Answer::Choice(ONE_FOLDER.to_owned()));
    let Outcome::Read(key, Query::CommitFolders { revision }) = save(&context, &answers) else {
        panic!("the folders are read")
    };
    assert_eq!((key, revision.as_str()), (FOLDERS, commit('3').as_str()));
    let answers = answers.with(FOLDERS, Answer::Names(vec![String::from("docs")]));
    assert_eq!(question(save(&context, &answers)).key, FOLDER);
    let answers = answers
        .with(FOLDER, Answer::Choice(String::from("docs")))
        .with(FORMAT, Answer::Choice(TAR_GZ.to_owned()))
        .with(INSIDE, Answer::Text(String::new()))
        .with(PATH, Answer::Path(PathBuf::from("/out/a.tar.gz")));
    let line = lines(&plan(save(&context, &answers))).join("");
    assert!(
        line.starts_with("git archive --format=tar.gz -o /out/a.tar.gz "),
        "{line}"
    );
    assert!(line.ends_with(" -- docs"), "{line}");
    assert!(!line.contains("--prefix"), "{line}");
}

#[test]
fn archive_save__a_stash_and_the_draft_are_refused() {
    let context = context(state(), Target::Head);
    assert!(matches!(save(&context, &Answers::new()), Outcome::Refuse(_)));
}
