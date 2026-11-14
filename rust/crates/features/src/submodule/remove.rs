//! `submodule.remove`: deletes a submodule from the repository, its folder, its entry in `.gitmodules`, its record and
//! its clone in the git directory, the removal staged.

use std::path::{Path, PathBuf};

use model::contract::{AnswerKey, Answers, Context, Danger, Outcome, Plan, Query, Step, Text, Undo};

use super::{submodule_step, target_submodule};

/// The git directory of the repository that has the submodule, where its clone is kept under `modules/`.
pub const GIT_FOLDER: AnswerKey = AnswerKey("git-folder");

/// Runs `git [-C <parent>] submodule deinit --force -- <path>`, then `git [-C <parent>] rm --force -- <path>`, then
/// deletes its clone, `<git directory>/modules/<name>`, when its name keeps it inside `modules`: a name that leads out
/// of it is no clone to delete. Destructive, and asked first with a warning.
///
/// Feature: submodule.remove
#[must_use]
pub fn remove(context: &Context, answers: &Answers) -> Outcome {
    let submodule = match target_submodule(context) {
        Ok(submodule) => submodule,
        Err(refusal) => return *refusal,
    };
    let Some(git_folder) = answers.path(GIT_FOLDER) else {
        let parent = submodule.parent.as_deref().map_or_else(
            || context.repository.folder.clone(),
            |parent| on_disk_folder(&context.repository.folder, parent),
        );
        return Outcome::Read(GIT_FOLDER, Query::GitFolder { folder: parent });
    };
    let clone = clone_folder(git_folder, &submodule.name);
    let mut steps = vec![
        submodule_step(context, submodule, &["submodule", "deinit", "--force"]),
        submodule_step(context, submodule, &["rm", "--force"]),
    ];
    let deleted = clone
        .as_ref()
        .map_or_default(|clone| format!(" Gitardo then deletes its clone in {}.", clone.display()));
    steps.extend(clone.map(|path| Step::Delete { path }));
    let warning = format!(
        "Its folder is deleted, with the changes and the commits in it that are not pushed.{deleted} The removal is \
         staged along with .gitmodules: commit them to record it."
    );
    Outcome::Run(Plan {
        title: Text::from(format!("Remove submodule {}", submodule.folder)),
        steps,
        danger: Danger::Destructive(Text::from(warning)),
        undo: Undo::Impossible(Text::from(
            "Cannot be undone: its clone is deleted. What was pushed of it can be added again.",
        )),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

/// The clone of a submodule by its name in `.gitmodules`, under `modules` of a git directory; none for a name that
/// leads out of it: empty, absolute, with a drive, or with `..`.
fn clone_folder(git_folder: &Path, name: &str) -> Option<PathBuf> {
    let parts: Vec<&str> = name.split(['/', '\\']).collect();
    let is_inside = !name.is_empty()
        && !name.starts_with(['/', '\\'])
        && !name.contains(':')
        && parts
            .iter()
            .all(|part| !part.is_empty() && *part != "." && *part != "..");
    is_inside.then(|| {
        parts
            .iter()
            .fold(git_folder.join("modules"), |path, part| path.join(part))
    })
}

/// A folder from the top of the repository, as a path.
fn on_disk_folder(top: &Path, folder: &str) -> PathBuf {
    folder.split('/').fold(top.to_path_buf(), |path, part| path.join(part))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use model::contract::{Answer, Answers, Danger, Outcome, Query};

    use super::{GIT_FOLDER, remove};
    use crate::submodule::testing::{lib, nested, on};
    use crate::testing::{lines, plan};

    fn read(git_folder: &str) -> Answers {
        Answers::new().with(GIT_FOLDER, Answer::Path(PathBuf::from(git_folder)))
    }

    #[test]
    fn submodule_remove__on_lib_it_warns_then_deinitializes_removes_and_deletes_its_clone() {
        let context = on(lib());
        assert_eq!(
            remove(&context, &Answers::new()),
            Outcome::Read(
                GIT_FOLDER,
                Query::GitFolder {
                    folder: PathBuf::from("repository")
                }
            )
        );
        let plan = plan(remove(&context, &read("repository/.git")));
        let clone = PathBuf::from("repository/.git").join("modules").join("lib");
        assert_eq!(
            lines(&plan),
            [
                String::from("git submodule deinit --force -- lib"),
                String::from("git rm --force -- lib"),
                format!("delete {}", clone.display()),
            ]
        );
        assert!(matches!(plan.danger, Danger::Destructive(_)));
    }

    #[test]
    fn submodule_remove__a_submodule_inside_another_is_removed_from_it_with_the_git_folder_of_that_one() {
        let context = on(nested());
        let Outcome::Read(_, Query::GitFolder { folder }) = remove(&context, &Answers::new()) else {
            panic!("the git folder of vendor/a is read first");
        };
        assert_eq!(folder, PathBuf::from("repository").join("vendor").join("a"));
        let plan = plan(remove(&context, &read("repository/.git/modules/a")));
        assert_eq!(
            lines(&plan)[..2],
            [
                "git -C vendor/a submodule deinit --force -- b",
                "git -C vendor/a rm --force -- b"
            ]
        );
    }

    #[test]
    fn submodule_remove__a_name_that_leads_out_of_modules_has_no_clone_deleted() {
        for name in ["../../x", "/etc", "C:/x", "a/../../b", ""] {
            let mut escaping = lib();
            escaping.name = name.to_owned();
            let plan = plan(remove(&on(escaping), &read("repository/.git")));
            assert_eq!(lines(&plan).len(), 2, "{name}: {:?}", lines(&plan));
            assert!(lines(&plan).iter().all(|line| !line.starts_with("delete")));
        }
    }
}
