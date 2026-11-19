#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::PathBuf;

use model::contract::{Answer, Answers, FollowUp, Outcome, Query, QuestionKind, Target};
use model::state::RepositoryState;

use super::{
    BRANCH, CLIPBOARD, DEPTH, HAS_FILES, NAME, OPTIONS, OTHER_BRANCH, PARENT, SHALLOW, START_FOLDER, SUBMODULES, TAKEN,
    URL, clone,
};
use crate::testing::{context, lines, plan, question};

const LIB: &str = "https://example.com/acme/lib.git";

/// The context of a clone: no repository, the folder the dialogs start in.
fn nowhere() -> model::contract::Context {
    context(
        RepositoryState {
            folder: PathBuf::from("home"),
            ..RepositoryState::default()
        },
        Target::Head,
    )
}

/// The answers up to the options, for a URL cloned into `work`, with nothing taken there.
fn answered(url: &str, options: &[&str]) -> Answers {
    Answers::new()
        .with(CLIPBOARD, Answer::Clipboard(None))
        .with(URL, Answer::Text(url.to_owned()))
        .with(PARENT, Answer::Path(PathBuf::from("work")))
        .with(TAKEN, Answer::Names(Vec::new()))
        .with(NAME, Answer::Text(String::from("lib")))
        .with(
            OPTIONS,
            Answer::Choices(options.iter().map(|option| (*option).to_owned()).collect()),
        )
        .with(HAS_FILES, Answer::Flag(false))
}

#[test]
fn repo_clone__with_nothing_ticked_but_the_submodules_it_clones_in_the_folder_chosen_and_opens_the_clone() {
    let plan = plan(clone(&nowhere(), &answered(LIB, &[SUBMODULES])));
    assert_eq!(
        lines(&plan),
        ["git clone --progress --recurse-submodules -- https://example.com/acme/lib.git lib"]
    );
    let model::contract::Step::Git { directory, .. } = &plan.steps[0] else {
        panic!("a git step");
    };
    assert_eq!(directory, &PathBuf::from("work"));
    assert_eq!(plan.title.as_str(), "Clone https://example.com/acme/lib.git into lib");
    let folder = PathBuf::from("work").join("lib");
    assert_eq!(
        plan.after[..2],
        [
            FollowUp::RememberCloneFolder(PathBuf::from("work")),
            FollowUp::OpenRepository(folder)
        ]
    );
}

#[test]
fn repo_clone__with_a_branch_and_50_commits_it_clones_that_branch_shallow() {
    let answers = answered(LIB, &[OTHER_BRANCH, SHALLOW, SUBMODULES]);
    let asked = question(clone(&nowhere(), &answers));
    assert_eq!(asked.key, BRANCH);
    let answers = answers.with(BRANCH, Answer::Text(String::from("stable")));
    let asked = question(clone(&nowhere(), &answers));
    assert_eq!(asked.key, DEPTH);
    assert_eq!(asked.default, Some(Answer::Text(String::from("1"))));
    assert_eq!(
        asked.detail.map(|detail| detail.to_string()).as_deref(),
        Some("Only this branch is fetched, as git does for a shallow clone.")
    );
    let answers = answers.with(DEPTH, Answer::Text(String::from("50")));
    assert_eq!(
        lines(&plan(clone(&nowhere(), &answers))),
        [
            "git clone --progress --branch stable --depth 50 --recurse-submodules -- https://example.com/acme/lib.git lib"
        ]
    );
}

#[test]
fn repo_clone__a_name_whose_folder_holds_a_file_is_refused_while_typed_and_an_empty_folder_is_taken() {
    let answers = answered(LIB, &[]).with(TAKEN, Answer::Names(vec![String::from("lib")]));
    let answers = Answers::new()
        .with(CLIPBOARD, Answer::Clipboard(None))
        .with(URL, Answer::Text(String::from(LIB)))
        .with(PARENT, Answer::Path(PathBuf::from("work")))
        .with(TAKEN, answers.get(TAKEN).cloned().unwrap());
    let asked = question(clone(&nowhere(), &answers));
    assert_eq!(asked.default, Some(Answer::Text(String::from("lib"))));
    let QuestionKind::Text { validation, .. } = asked.kind else {
        panic!("the name is typed");
    };
    let full = PathBuf::from("work").join("lib");
    assert_eq!(
        validation.problem("lib").map(|text| text.to_string()),
        Some(format!("{} is not empty", full.display()))
    );
    // An empty folder of that name is not among the names taken, and is taken.
    assert_eq!(validation.problem("tools"), None);
    assert_eq!(
        validation.hint("tools").map(|text| text.to_string()),
        Some(format!("In {}", PathBuf::from("work").join("tools").display()))
    );
    let Outcome::Refuse(refusal) = clone(&nowhere(), &answered(LIB, &[]).with(HAS_FILES, Answer::Flag(true))) else {
        panic!("a folder with files by the time it runs is refused");
    };
    assert_eq!(refusal.message.to_string(), format!("{} is not empty.", full.display()));
}

#[test]
fn repo_clone__an_option_as_the_url_and_a_name_that_starts_with_a_dash_are_refused_while_typed() {
    let asked = question(clone(
        &nowhere(),
        &Answers::new().with(CLIPBOARD, Answer::Clipboard(None)),
    ));
    let QuestionKind::Text { validation, .. } = asked.kind else {
        panic!("the URL is typed");
    };
    assert_eq!(
        validation
            .problem("--upload-pack=x")
            .map(|text| text.to_string())
            .as_deref(),
        Some("That is not a URL")
    );
    assert_eq!(
        validation.problem("").map(|text| text.to_string()).as_deref(),
        Some("Enter a URL")
    );
    let answers = Answers::new()
        .with(CLIPBOARD, Answer::Clipboard(None))
        .with(URL, Answer::Text(String::from(LIB)))
        .with(PARENT, Answer::Path(PathBuf::from("work")))
        .with(TAKEN, Answer::Names(Vec::new()));
    let QuestionKind::Text { validation, .. } = question(clone(&nowhere(), &answers)).kind else {
        panic!("the name is typed");
    };
    assert_eq!(
        validation.problem("-lib").map(|text| text.to_string()).as_deref(),
        Some("Enter the name of one folder")
    );
}

#[test]
fn repo_clone__a_path_on_disk_does_not_offer_a_shallow_clone() {
    let offered = |url: &str| {
        let answers = answered(url, &[]);
        let answers = Answers::new()
            .with(CLIPBOARD, Answer::Clipboard(None))
            .with(URL, answers.get(URL).cloned().unwrap())
            .with(PARENT, Answer::Path(PathBuf::from("work")))
            .with(TAKEN, Answer::Names(Vec::new()))
            .with(NAME, Answer::Text(String::from("lib")));
        let QuestionKind::PickMany { choices } = question(clone(&nowhere(), &answers)).kind else {
            panic!("the options are ticked");
        };
        choices.into_iter().map(|choice| choice.identifier).collect::<Vec<_>>()
    };
    assert_eq!(offered("C:\\repos\\lib"), [OTHER_BRANCH, SUBMODULES]);
    assert_eq!(offered("/srv/repos/lib"), [OTHER_BRANCH, SUBMODULES]);
    assert_eq!(offered(LIB), [OTHER_BRANCH, SHALLOW, SUBMODULES]);
    assert_eq!(
        offered("git@github.com:owner/lib.git"),
        [OTHER_BRANCH, SHALLOW, SUBMODULES]
    );
}

#[test]
fn repo_clone__backing_out_of_any_question_runs_nothing() {
    // Each question is asked until it is answered: a question backed out of is never answered, and the engine runs
    // nothing; here every question asked before the plan is one the user can back out of.
    let mut answers = Answers::new().with(CLIPBOARD, Answer::Clipboard(Some(String::from(LIB))));
    let url = question(clone(&nowhere(), &answers));
    assert_eq!(
        url.default,
        Some(Answer::Text(String::from(LIB))),
        "the clipboard is the default"
    );
    answers = answers.with(URL, Answer::Text(String::from(LIB)));
    let parent = question(clone(
        &nowhere(),
        &answers
            .clone()
            .with(START_FOLDER, Answer::Path(PathBuf::from("C:/work"))),
    ));
    let QuestionKind::OpenFolder(dialog) = parent.kind else {
        panic!("the folder is chosen in the system's dialog");
    };
    assert_eq!(
        dialog.folder,
        PathBuf::from("C:/work"),
        "it starts where the last clone went"
    );
    answers = answers.with(PARENT, Answer::Path(PathBuf::from("work")));
    assert_eq!(
        clone(&nowhere(), &answers),
        Outcome::Read(
            TAKEN,
            Query::TakenNames {
                folder: PathBuf::from("work")
            }
        )
    );
    answers = answers.with(TAKEN, Answer::Names(Vec::new()));
    assert_eq!(question(clone(&nowhere(), &answers)).key, NAME);
    answers = answers.with(NAME, Answer::Text(String::from("lib")));
    let options = question(clone(&nowhere(), &answers));
    assert_eq!(options.default, Some(Answer::Choices(vec![String::from(SUBMODULES)])));
}
