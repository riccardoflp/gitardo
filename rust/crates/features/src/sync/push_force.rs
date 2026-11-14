//! `sync.push-force`: replaces the remote branch with the branch checked out, whatever it has.

use model::contract::{Answers, Context, Outcome};

use super::forced::forced_push;

/// Pushes with `--force`, warning of the commits only the remote branch has as of the last fetch.
///
/// Feature: sync.push-force
#[must_use]
pub fn push_force(context: &Context, answers: &Answers) -> Outcome {
    forced_push(context, answers, "--force")
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::push_force;
    use crate::sync::forced::OVERWRITTEN;
    use crate::testing::{context, lines, plan, state, tracking};
    use model::contract::{Answer, Answers, Danger, Outcome, Query, Target};
    use model::remote::{Overwritten, OverwrittenCommit};

    fn context_on_main() -> model::contract::Context {
        let mut state = state();
        state.remotes.push(String::from("origin"));
        if let Some(main) = state.branches.first_mut() {
            main.upstream = Some(tracking("origin", "main"));
        }
        context(state, Target::Head)
    }

    fn overwritten(count: usize) -> Answers {
        let commits = (0..count.min(10))
            .map(|index| OverwrittenCommit {
                short_id: format!("abc{index:04}"),
                subject: format!("Change {index}"),
            })
            .collect();
        let overwritten = Overwritten {
            commits,
            has_more: count > 10,
        };
        Answers::new().with(OVERWRITTEN, Answer::Overwritten(overwritten))
    }

    #[test]
    fn sync_push_force__asks_with_what_it_overwrites_then_forces() {
        let context = context_on_main();
        let Outcome::Read(_, Query::Overwritten { upstream }) = push_force(&context, &Answers::new()) else {
            panic!("what is overwritten is read first")
        };
        assert_eq!(upstream, "refs/remotes/origin/main");
        let plan = plan(push_force(&context, &overwritten(2)));
        assert_eq!(lines(&plan), ["git push --force origin main"]);
        let Danger::Destructive(warning) = plan.danger else {
            panic!("a forced push warns")
        };
        assert_eq!(
            warning.as_str(),
            "These commits on `origin/main` are not in `main` and will be overwritten:\nabc0000 Change 0\nabc0001 \
             Change 1"
        );
    }

    #[test]
    fn sync_push_force__twelve_commits_list_ten_and_more() {
        let plan = plan(push_force(&context_on_main(), &overwritten(12)));
        let Danger::Destructive(warning) = plan.danger else {
            panic!("a forced push warns")
        };
        let lines: Vec<&str> = warning.as_str().lines().collect();
        assert_eq!(lines.len(), 12);
        assert_eq!(lines.last(), Some(&"\u{2026}and more"));
    }
}
