//! `sync.push-force-with-lease`: replaces the remote branch with the branch checked out, unless it moved since the last
//! fetch.

use model::contract::{Answers, Context, Outcome};

use super::forced::forced_push;

/// Pushes with `--force-with-lease`, warning of the commits only the remote branch has as of the last fetch; git
/// refuses when the remote branch is not where the last fetch saw it.
///
/// Feature: sync.push-force-with-lease
#[must_use]
pub fn push_force_with_lease(context: &Context, answers: &Answers) -> Outcome {
    forced_push(context, answers, "--force-with-lease")
}

#[cfg(test)]
mod tests {
    use super::push_force_with_lease;
    use crate::sync::forced::OVERWRITTEN;
    use crate::testing::{context, lines, plan, state, tracking};
    use model::contract::{Answer, Answers, Danger, Target, Text};
    use model::remote::Overwritten;

    #[test]
    fn says_when_nothing_is_overwritten() {
        let mut state = state();
        state.remotes.push(String::from("origin"));
        if let Some(main) = state.branches.first_mut() {
            main.upstream = Some(tracking("origin", "main"));
        }
        let answers = Answers::new().with(OVERWRITTEN, Answer::Overwritten(Overwritten::default()));
        let plan = plan(push_force_with_lease(&context(state, Target::Head), &answers));
        assert_eq!(lines(&plan), ["git push --force-with-lease origin main"]);
        assert_eq!(
            plan.danger,
            Danger::Destructive(Text::from(
                "As of the last fetch, every commit on `origin/main` is also in `main`."
            ))
        );
    }
}
