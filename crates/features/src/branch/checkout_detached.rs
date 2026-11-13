//! `branch.checkout-detached`: checks out a commit on a detached `HEAD`, to look at it or build it.

use model::contract::{Answers, Context, Outcome, Step, Target};

use super::checkout::checkout_plan;
use super::submodules::update_after_checkout;
use super::target::refuse;

/// Checks out the commit it was invoked on, by its full hash, on a detached `HEAD`, then updates the submodules when
/// `submodules.updateOnCheckout` asks for it.
///
/// Feature: branch.checkout-detached
#[must_use]
pub fn checkout_detached(context: &Context, answers: &Answers) -> Outcome {
    let Target::Commit(commit) = &context.target else {
        return refuse("Choose a commit first.");
    };
    let head = &context.repository.head;
    if head.commit.as_ref() == Some(commit) {
        return refuse(&format!("{} is checked out already.", commit.short()));
    }
    let steps = vec![Step::git(
        context.repository.folder.clone(),
        ["checkout", "--detach", commit.as_str()],
    )];
    let title = format!("Check out {} (detached)", commit.short());
    update_after_checkout(context, answers, commit.as_str(), checkout_plan(title, steps))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::checkout_detached;
    use crate::branch::submodules::none;
    use crate::testing::{commit, context, lines, plan, state};
    use model::contract::{Answers, Outcome, Target};

    #[test]
    fn branch_checkout_detached__checks_out_the_full_hash_on_a_detached_head() {
        let context = context(state(), Target::Commit(commit('b')));
        let plan = plan(checkout_detached(&context, &none(Answers::new())));
        assert_eq!(lines(&plan), [format!("git checkout --detach {}", commit('b'))]);
        assert_eq!(plan.title.as_str(), "Check out bbbbbbb (detached)");
    }

    #[test]
    fn branch_checkout_detached__refuses_the_commit_checked_out() {
        let context = context(state(), Target::Commit(commit('a')));
        assert!(matches!(
            checkout_detached(&context, &Answers::new()),
            Outcome::Refuse(_)
        ));
    }
}
