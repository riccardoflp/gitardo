//! `changes.stage`: puts the changes of a file in the index, for the next commit.

use model::contract::{Answers, Context, Danger, Outcome};

use super::files::{plan, target_file, with_files};

/// Stages a file of the Changes page: `git add -- <path>`, the path named as itself. A file deleted has its deletion
/// staged.
///
/// Feature: changes.stage
#[must_use]
pub fn stage(context: &Context, _answers: &Answers) -> Outcome {
    let path = match target_file(context) {
        Ok(path) => path,
        Err(refusal) => return Outcome::Refuse(refusal),
    };
    let folder = context.repository.folder.clone();
    let steps = vec![with_files(folder, &["add"], [path])];
    Outcome::Run(plan(
        format!("Stage {path}"),
        steps,
        Danger::None,
        "Unstaging it takes it back.",
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::stage;
    use crate::testing::{context, lines, plan, state};
    use model::contract::{Answers, Outcome, Target};

    fn staged(path: &str) -> Vec<String> {
        lines(&plan(stage(
            &context(state(), Target::File(path.to_owned())),
            &Answers::new(),
        )))
    }

    #[test]
    fn changes_stage__an_untracked_file_is_added() {
        assert_eq!(staged("new.txt"), ["git add -- new.txt"]);
    }

    #[test]
    fn changes_stage__a_half_ticked_file_is_added_whole() {
        // The rest of a file partly staged is staged by the same command: git adds the file as it is on disk.
        assert_eq!(staged("a.txt"), ["git add -- a.txt"]);
    }

    #[test]
    fn changes_stage__a_name_that_is_a_pattern_is_named_literally() {
        assert_eq!(staged("pages/[id].tsx"), ["git add -- :(literal)pages/[id].tsx"]);
    }

    #[test]
    fn changes_stage__a_deleted_file_has_its_deletion_staged() {
        // `git add` of a file gone from disk stages its deletion.
        assert_eq!(staged("gone.txt"), ["git add -- gone.txt"]);
    }

    #[test]
    fn refuses_without_a_file() {
        assert!(matches!(
            stage(&context(state(), Target::Head), &Answers::new()),
            Outcome::Refuse(_)
        ));
    }
}
