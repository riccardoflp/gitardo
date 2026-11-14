//! `submodule.initialize`: clones a submodule that is not initialized and checks out the commit recorded for it, with
//! its own submodules.

use model::contract::{Answers, Context, Danger, FollowUp, Outcome, Plan, Text, Undo};

use super::{refusal, submodule_step, target_submodule};

/// Runs `git [-C <parent>] submodule update --init --recursive -- <path>` on a submodule whose folder has no
/// repository.
///
/// Feature: submodule.initialize
#[must_use]
pub fn initialize(context: &Context, _answers: &Answers) -> Outcome {
    let submodule = match target_submodule(context) {
        Ok(submodule) => submodule,
        Err(refusal) => return *refusal,
    };
    if submodule.checkout.is_some() {
        return *refusal(&format!("{} is initialized already.", submodule.folder));
    }
    Outcome::Run(Plan {
        title: Text::from(format!("Initialize {}", submodule.folder)),
        steps: vec![submodule_step(
            context,
            submodule,
            &["submodule", "update", "--init", "--recursive"],
        )],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Not taken back by Undo: Deinitialize empties it again.")),
        after: Vec::new(),
        on_failure: vec![FollowUp::AllowFileProtocol],
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answers, Outcome};

    use super::initialize;
    use crate::submodule::testing::{lib, on};
    use crate::testing::{lines, plan};

    #[test]
    fn submodule_initialize__clones_lib_with_its_own_submodules() {
        let mut empty = lib();
        empty.checkout = None;
        let plan = plan(initialize(&on(empty), &Answers::new()));
        assert_eq!(lines(&plan), ["git submodule update --init --recursive -- lib"]);
        assert!(matches!(initialize(&on(lib()), &Answers::new()), Outcome::Refuse(_)));
    }
}
