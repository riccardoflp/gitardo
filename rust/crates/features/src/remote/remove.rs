//! `remote.remove`: removes a remote and its remote-tracking branches from this repository.

use model::contract::{Answers, Context, Danger, Outcome, Plan, Step, Text, Undo};

use super::target_remote;

/// Removes the remote it was invoked on, warning first that its remote-tracking branches go and the branches that
/// follow them lose their upstream.
///
/// Feature: remote.remove
#[must_use]
pub fn remove(context: &Context, _answers: &Answers) -> Outcome {
    let name = match target_remote(context) {
        Ok(name) => name,
        Err(refusal) => return *refusal,
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Remove remote {name}")),
        steps: vec![Step::git(context.repository.folder.clone(), ["remote", "remove", name])],
        danger: Danger::Destructive(Text::from(format!(
            "Its remote-tracking branches (`{name}`/\u{2026}) are deleted from this repository, and local branches that \
             follow them lose their upstream. Nothing changes on the remote itself."
        ))),
        undo: Undo::Impossible(Text::from(
            "Adding the remote again and fetching it brings its branches back, but not the upstreams.",
        )),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::remove;
    use crate::testing::{context, lines, plan, state};
    use model::contract::{Answers, Danger, Target};

    #[test]
    fn remote_remove__asks_with_a_warning_whatever_command_preview_says() {
        let context = context(state(), Target::Remote(String::from("upstream")));
        let plan = plan(remove(&context, &Answers::new()));
        assert_eq!(lines(&plan), ["git remote remove upstream"]);
        assert!(matches!(plan.danger, Danger::Destructive(_)));
    }
}
