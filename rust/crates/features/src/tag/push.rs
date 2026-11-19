//! `tag.push`: sends a tag to the remote.

use model::contract::{Answers, Context, Danger, FollowUp, Outcome, Plan, Step, Text, Undo};
use model::tag::TagsOnRemote;

use crate::branch::target::{self, refuse};

/// Pushes the tag it was invoked on to the remote a tag goes to: `origin` when the repository has it, otherwise the
/// first remote git lists (`remote.default`).
///
/// Feature: tag.push
#[must_use]
pub fn push(context: &Context, _answers: &Answers) -> Outcome {
    let tag = match target::tag(context) {
        Ok(tag) => tag,
        Err(refusal) => return *refusal,
    };
    let Some(remote) = model::remote::default(&context.repository.remotes) else {
        return refuse("This repository has no remote.");
    };
    let name = tag.name.as_str();
    Outcome::Run(Plan {
        title: Text::from(format!("Push tag {name} to {remote}")),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["push", remote.as_str(), &format!("refs/tags/{name}")],
        )],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("A push changes the remote, which Undo does not take back.")),
        after: vec![FollowUp::TagsOnRemote(Box::new(TagsOnRemote::pushed(
            &remote,
            vec![name.to_owned()],
        )))],
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::push;
    use crate::testing::{context, lines, plan, state, tag};
    use model::contract::{Answers, Outcome, Target};

    fn pushed(remotes: &[&str]) -> Outcome {
        let mut state = state();
        state.tags.push(tag("v1.0.0", 'b'));
        state.remotes = remotes.iter().map(|remote| (*remote).to_owned()).collect();
        push(&context(state, Target::Tag(String::from("v1.0.0"))), &Answers::new())
    }

    #[test]
    fn tag_push__pushes_to_origin_among_the_remotes() {
        assert_eq!(
            lines(&plan(pushed(&["upstream", "origin"]))),
            ["git push origin refs/tags/v1.0.0"]
        );
    }

    #[test]
    fn tag_push__pushes_to_the_first_remote_without_origin() {
        assert_eq!(lines(&plan(pushed(&["a", "b"]))), ["git push a refs/tags/v1.0.0"]);
    }

    #[test]
    fn tag_push__refuses_without_a_remote() {
        assert!(matches!(
            pushed(&[]),
            Outcome::Refuse(refusal) if refusal.message.as_str() == "This repository has no remote."
        ));
    }
}
