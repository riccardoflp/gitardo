//! `tag.checkout`: checks out the commit of a tag on a detached `HEAD`.

use model::branch::{RefKind, RefNames, ref_argument};
use model::contract::{Answers, Context, Outcome, Step};

use crate::branch::submodules::update_after_checkout;
use crate::branch::target::{self, refuse};

/// Checks out the tag it was invoked on, detached, named in full when a branch has its name, then updates the
/// submodules when `submodules.updateOnCheckout` asks for it.
///
/// Feature: tag.checkout
#[must_use]
pub fn checkout(context: &Context, answers: &Answers) -> Outcome {
    let tag = match target::tag(context) {
        Ok(tag) => tag,
        Err(refusal) => return *refusal,
    };
    if context.repository.head.commit.as_ref() == Some(&tag.commit) {
        return refuse(&format!("The commit of {} is checked out already.", tag.name));
    }
    let argument = ref_argument(RefKind::Tag, &tag.name, &RefNames::of(&context.repository));
    let mut steps = vec![Step::git(
        context.repository.folder.clone(),
        ["checkout", "--detach", argument.as_str()],
    )];
    match update_after_checkout(context, answers, &argument) {
        Ok(update) => steps.extend(update),
        Err(read) => return *read,
    }
    let title = format!("Check out tag {} (detached)", tag.name);
    Outcome::Run(crate::branch::checkout_plan(title, steps))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::checkout;
    use crate::branch::submodules::none;
    use crate::testing::{branch, context, lines, plan, state, tag};
    use model::contract::{Answers, Outcome, Target};

    fn on_tag(state: model::state::RepositoryState) -> Outcome {
        checkout(
            &context(state, Target::Tag(String::from("v1.0.0"))),
            &none(Answers::new()),
        )
    }

    #[test]
    fn tag_checkout__checks_out_the_tag_detached_in_full_when_a_branch_has_its_name() {
        let mut state = state();
        state.tags.push(tag("v1.0.0", 'b'));
        assert_eq!(lines(&plan(on_tag(state.clone()))), ["git checkout --detach v1.0.0"]);
        state.branches.push(branch("v1.0.0", 'c', false));
        assert_eq!(lines(&plan(on_tag(state))), ["git checkout --detach refs/tags/v1.0.0"]);
    }

    #[test]
    fn tag_checkout__refuses_a_tag_on_the_commit_checked_out() {
        let mut state = state();
        state.tags.push(tag("v1.0.0", 'a'));
        assert!(matches!(on_tag(state), Outcome::Refuse(_)));
    }
}
