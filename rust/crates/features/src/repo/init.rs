//! `repo.init`: makes an empty repository in a folder, with the first branch git's configuration says or the one typed,
//! and opens it in a tab.

use std::ffi::OsString;
use std::path::Path;

use model::contract::{
    Answer, AnswerKey, Answers, Context, Danger, FileQuestion, FollowUp, Outcome, Plan, Query, Question, QuestionKind,
    Refusal, Step, Text, Undo, Validation,
};

/// The folder the repository is made in: chosen in the system's dialog, or given by the timeline of a workspace with
/// no repository.
pub const FOLDER: AnswerKey = AnswerKey("folder");
/// The top folder of the repository the folder is in, if any.
const AROUND: AnswerKey = AnswerKey("around");
/// Whether a repository is made inside the one around the folder.
pub const INSIDE: AnswerKey = AnswerKey("inside");
/// The first branch `init.defaultBranch` names, if any.
const DEFAULT_BRANCH: AnswerKey = AnswerKey("default-branch");
/// The first branch typed, when git's configuration names none.
pub const BRANCH: AnswerKey = AnswerKey("branch");

/// Asks the folder, refuses one that is a repository, asks about one inside a repository, asks the first branch when
/// `init.defaultBranch` is not set, then runs `git init [--initial-branch=<name>]` in it.
///
/// Feature: repo.init
#[must_use]
pub fn init(context: &Context, answers: &Answers) -> Outcome {
    let Some(Answer::Path(folder)) = answers.get(FOLDER) else {
        return Outcome::Ask(folder_question(context));
    };
    let Some(around) = answers.paths(AROUND) else {
        return Outcome::Read(AROUND, Query::RepositoryAround { folder: folder.clone() });
    };
    if let Some(top) = around.first() {
        if same_folder(top, folder) {
            return Outcome::Refuse(Refusal::saying(&format!(
                "{} is a repository already.",
                folder.display()
            )));
        }
        if !answers.is_confirmed(INSIDE) {
            return Outcome::Ask(inside_question(folder, top));
        }
    }
    let Some(default_branch) = answers.names(DEFAULT_BRANCH) else {
        return Outcome::Read(DEFAULT_BRANCH, Query::DefaultBranch { folder: folder.clone() });
    };
    let branch = match answers.text(BRANCH) {
        None if default_branch.is_empty() => return Outcome::Ask(branch_question()),
        branch => branch.map(str::trim).filter(|_| default_branch.is_empty()),
    };
    let mut arguments = vec![OsString::from("init")];
    if let Some(branch) = branch {
        arguments.push(OsString::from(format!("--initial-branch={branch}")));
    }
    Outcome::Run(Plan {
        title: Text::from(format!("Create a repository in {}", folder.display())),
        steps: vec![Step::Git {
            arguments,
            directory: folder.clone(),
            input: None,
        }],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "Not taken back by Undo: the folder .git stays, and deleting it is yours.",
        )),
        after: vec![
            FollowUp::OpenRepository(folder.clone()),
            FollowUp::Say(Text::from(format!("Created a repository in {}", folder.display()))),
        ],
        on_failure: Vec::new(),
    })
}

/// Whether two folders are the same, as git and the system's dialog write them: with `/` or `\`, with a `/` at the end
/// or not, and on Windows whatever the case.
fn same_folder(left: &Path, right: &Path) -> bool {
    let words = |path: &Path| {
        let words = path.to_string_lossy().replace('\\', "/");
        let words = words.trim_end_matches('/').to_owned();
        if cfg!(windows) { words.to_lowercase() } else { words }
    };
    words(left) == words(right)
}

/// The folder, in the system's dialog.
fn folder_question(context: &Context) -> Question {
    Question {
        key: FOLDER,
        kind: QuestionKind::OpenFolder(FileQuestion {
            folder: context.repository.folder.clone(),
            file_name: None,
            filters: Vec::new(),
        }),
        title: Text::from("Folder to create a repository in"),
        detail: None,
        default: None,
    }
}

/// Whether to make a repository inside another.
fn inside_question(folder: &Path, top: &Path) -> Question {
    Question {
        key: INSIDE,
        kind: QuestionKind::Confirm {
            accept: Text::from("Create"),
        },
        title: Text::from(format!(
            "{} is inside the repository {}. Create another repository in it?",
            folder.display(),
            top.display()
        )),
        detail: None,
        default: None,
    }
}

/// The first branch, `main` by default.
fn branch_question() -> Question {
    Question {
        key: BRANCH,
        kind: QuestionKind::Text {
            placeholder: None,
            validation: Validation::ReferenceName {
                taken: Vec::new(),
                taken_message: Text::from(""),
            },
        },
        title: Text::from("Name of the first branch"),
        detail: None,
        default: Some(Answer::Text(String::from("main"))),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use model::contract::{Answer, Answers, Outcome, Query, QuestionKind, Target};
    use model::state::RepositoryState;

    use super::{AROUND, BRANCH, DEFAULT_BRANCH, FOLDER, INSIDE, init};
    use crate::testing::{context, lines, plan, question};

    fn nowhere() -> model::contract::Context {
        context(
            RepositoryState {
                folder: PathBuf::from("home"),
                ..RepositoryState::default()
            },
            Target::Head,
        )
    }

    fn in_new() -> Answers {
        Answers::new().with(FOLDER, Answer::Path(PathBuf::from("work/new")))
    }

    #[test]
    fn repo_init__with_init_default_branch_set_it_asks_no_branch_and_runs_git_init_there() {
        let answers = in_new();
        assert_eq!(
            init(&nowhere(), &answers),
            Outcome::Read(
                AROUND,
                Query::RepositoryAround {
                    folder: PathBuf::from("work/new")
                }
            )
        );
        let answers = answers.with(AROUND, Answer::Paths(Vec::new()));
        assert_eq!(
            init(&nowhere(), &answers),
            Outcome::Read(
                DEFAULT_BRANCH,
                Query::DefaultBranch {
                    folder: PathBuf::from("work/new")
                }
            )
        );
        let plan = plan(init(
            &nowhere(),
            &answers.with(DEFAULT_BRANCH, Answer::Names(vec![String::from("trunk")])),
        ));
        assert_eq!(lines(&plan), ["git init"]);
        assert_eq!(
            plan.title.to_string(),
            format!("Create a repository in {}", PathBuf::from("work/new").display())
        );
    }

    #[test]
    fn repo_init__without_init_default_branch_it_asks_the_branch_main_first_and_release_runs_initial_branch() {
        let answers = in_new()
            .with(AROUND, Answer::Paths(Vec::new()))
            .with(DEFAULT_BRANCH, Answer::Names(Vec::new()));
        let asked = question(init(&nowhere(), &answers));
        assert_eq!(asked.key, BRANCH);
        assert_eq!(asked.default, Some(Answer::Text(String::from("main"))));
        let plan = plan(init(
            &nowhere(),
            &answers.with(BRANCH, Answer::Text(String::from("release"))),
        ));
        assert_eq!(lines(&plan), ["git init --initial-branch=release"]);
    }

    #[test]
    fn repo_init__a_dot_dot_as_the_first_branch_is_refused_while_typed() {
        let answers = in_new()
            .with(AROUND, Answer::Paths(Vec::new()))
            .with(DEFAULT_BRANCH, Answer::Names(Vec::new()));
        let QuestionKind::Text { validation, .. } = question(init(&nowhere(), &answers)).kind else {
            panic!("the branch is typed");
        };
        assert!(validation.problem("a..b").is_some());
        assert_eq!(validation.problem("main"), None);
    }

    #[test]
    fn repo_init__a_folder_that_is_a_repository_is_refused_and_nothing_runs() {
        let answers = in_new().with(AROUND, Answer::Paths(vec![PathBuf::from("work/new/")]));
        let Outcome::Refuse(refusal) = init(&nowhere(), &answers) else {
            panic!("a repository is refused");
        };
        assert_eq!(
            refusal.message.to_string(),
            format!("{} is a repository already.", PathBuf::from("work/new").display())
        );
    }

    #[test]
    fn repo_init__a_folder_inside_a_repository_asks_first() {
        let answers = in_new().with(AROUND, Answer::Paths(vec![PathBuf::from("work")]));
        let asked = question(init(&nowhere(), &answers));
        assert_eq!(asked.key, INSIDE);
        assert!(matches!(asked.kind, QuestionKind::Confirm { .. }));
        assert_eq!(
            asked.title.to_string(),
            format!(
                "{} is inside the repository {}. Create another repository in it?",
                PathBuf::from("work/new").display(),
                PathBuf::from("work").display()
            )
        );
        let answers = answers
            .with(INSIDE, Answer::Confirmed)
            .with(DEFAULT_BRANCH, Answer::Names(vec![String::from("main")]));
        assert_eq!(lines(&plan(init(&nowhere(), &answers))), ["git init"]);
    }
}
