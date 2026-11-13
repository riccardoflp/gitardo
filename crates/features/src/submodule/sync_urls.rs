//! `submodule.sync-urls`: copies the URLs of `.gitmodules` to the configuration of the submodules, and of theirs.

use model::contract::{Answers, Context, Danger, Outcome, Plan, Step, Text, Undo};

/// Runs `git submodule sync --recursive` at the top of the repository.
///
/// Feature: submodule.sync-urls
#[must_use]
pub fn sync_urls(context: &Context, _answers: &Answers) -> Outcome {
    Outcome::Run(Plan {
        title: Text::from("Synchronize the URLs of the submodules"),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["submodule", "sync", "--recursive"],
        )],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Not taken back by Undo: the URLs it replaced are not kept.")),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::Answers;

    use super::sync_urls;
    use crate::submodule::testing::{lib, on};
    use crate::testing::{lines, plan};

    #[test]
    fn submodule_sync_urls__runs_git_submodule_sync_recursive() {
        let plan = plan(sync_urls(&on(lib()), &Answers::new()));
        assert_eq!(lines(&plan), ["git submodule sync --recursive"]);
    }
}
