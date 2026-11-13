//! `compare.path-with`: compares a file or a folder as it is in the working tree with a branch, a tag or a commit, in a
//! comparison page narrowed to it.

use model::compare::CompareSide;
use model::contract::{Answers, Context, Outcome, Refusal, Target};

use super::{Picked, SideKeys, SideQuestion, comparison, opens, side};

/// Said of a file or a folder outside the repository, or of nothing.
const CHOOSE: &str = "Choose a file or a folder of this repository to compare.";

/// Asks for the branch, tag or commit to compare a file or a folder of the working tree with, then opens the
/// comparison of it with the **Working tree**, narrowed to that file or folder; the top folder of the repository is the
/// whole repository.
///
/// Feature: compare.path-with
#[must_use]
pub fn path_with(context: &Context, answers: &Answers) -> Outcome {
    let Target::Path(on_disk) = &context.target else {
        return Outcome::Refuse(Refusal::saying(CHOOSE));
    };
    let Some(path) = model::repo::path_in_folder(on_disk, &context.repository.folder) else {
        return Outcome::Refuse(Refusal::saying(CHOOSE));
    };
    let name = match path.rsplit('/').next() {
        Some(name) if !name.is_empty() => name.to_owned(),
        _ => on_disk
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned()),
    };
    let question = SideQuestion {
        title: format!("Compare {name} with"),
        placeholder: "The branch, tag or commit to compare the working tree with",
        chosen: None,
        may_offer_working_tree: false,
        recent: None,
    };
    match side(context, answers, SideKeys::FIRST, &question) {
        Ok(Picked::Side(first)) => {
            let narrowed = Some(path).filter(|path| !path.is_empty());
            opens(comparison(first, CompareSide::working_tree(), narrowed))
        }
        Ok(Picked::Recent(_)) => Outcome::Refuse(Refusal::saying("Choose a branch, tag or commit.")),
        Err(outcome) => *outcome,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use model::contract::{Answers, FollowUp, Outcome, QuestionKind, Target};

    use super::path_with;
    use crate::compare::FIRST;
    use crate::compare::testing::picked;
    use crate::file_history::testing::on_disk;
    use crate::testing::{branch, context, plan, question, state};

    /// A repository with `main` checked out and a branch `feature`.
    fn repository() -> model::state::RepositoryState {
        let mut state = state();
        state.branches.push(branch("feature", 'b', false));
        state
    }

    /// The first side, the second and the path of the comparison an outcome opens.
    fn opened(outcome: Outcome) -> (String, String, Option<String>) {
        let plan = plan(outcome);
        let Some(FollowUp::OpenComparison(target)) = plan.after.first() else {
            panic!("opens a comparison: {plan:?}");
        };
        (
            target.first.name.clone(),
            target.second.name.clone(),
            target.path.clone(),
        )
    }

    #[test]
    fn compare_path_with__asks_for_a_side_without_the_working_tree_and_narrows_to_the_file() {
        let context = context(repository(), Target::Path(on_disk("src/app.rs")));
        let asked = question(path_with(&context, &Answers::new()));
        assert_eq!(asked.title.as_str(), "Compare app.rs with");
        let QuestionKind::Pick { choices } = asked.kind else {
            panic!("a pick");
        };
        assert!(choices.iter().all(|choice| choice.identifier != "working-tree"));
        let answers = picked(&[(FIRST, "ref:main")]);
        assert_eq!(
            opened(path_with(&context, &answers)),
            (
                String::from("main"),
                String::from("Working tree"),
                Some(String::from("src/app.rs"))
            )
        );
    }

    #[test]
    fn compare_path_with__narrows_to_a_folder_and_compares_the_top_folder_whole() {
        let folder = context(repository(), Target::Path(on_disk("src")));
        let answers = picked(&[(FIRST, "ref:feature")]);
        assert_eq!(opened(path_with(&folder, &answers)).2, Some(String::from("src")));
        let top = context(repository(), Target::Path(on_disk("")));
        assert_eq!(opened(path_with(&top, &answers)).2, None);
        let outside = context(repository(), Target::Path(PathBuf::from("elsewhere/a.rs")));
        let Outcome::Refuse(refusal) = path_with(&outside, &answers) else {
            panic!("refuses a file outside");
        };
        assert_eq!(
            refusal.message.as_str(),
            "Choose a file or a folder of this repository to compare."
        );
    }
}
