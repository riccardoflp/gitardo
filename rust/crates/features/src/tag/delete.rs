//! `tag.delete`: deletes a tag from this repository; a remote that has it keeps it.

use model::contract::{Answers, Context, Danger, Outcome, Plan, Step, Text, Undo};

use crate::branch::target;

/// Deletes the tag it was invoked on with `git tag -d`, after a warning, since the tag and its message are gone from
/// this repository.
///
/// Feature: tag.delete
#[must_use]
pub fn delete(context: &Context, _answers: &Answers) -> Outcome {
    let tag = match target::tag(context) {
        Ok(tag) => tag,
        Err(refusal) => return *refusal,
    };
    let name = tag.name.as_str();
    Outcome::Run(Plan {
        title: Text::from(format!("Delete tag {name}")),
        steps: vec![Step::git(context.repository.folder.clone(), ["tag", "-d", name])],
        danger: Danger::Destructive(Text::from(format!(
            "The tag, on {}, is deleted from this repository, with its message if it has one. A remote that has it \
             keeps it.",
            tag.commit.short()
        ))),
        undo: Undo::Impossible(Text::from(
            "The reflog keeps no record of tags, and Gitardo does not remember deleted ones.",
        )),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::delete;
    use crate::testing::{context, lines, plan, state, tag};
    use model::contract::{Answers, Danger, Target};

    #[test]
    fn tag_delete__warns_then_deletes_the_tag_here_only() {
        let mut state = state();
        state.tags.push(tag("v1.0.0", 'b'));
        let plan = plan(delete(
            &context(state, Target::Tag(String::from("v1.0.0"))),
            &Answers::new(),
        ));
        assert_eq!(lines(&plan), ["git tag -d v1.0.0"]);
        assert!(matches!(&plan.danger, Danger::Destructive(reason) if reason.as_str().contains("bbbbbbb")));
    }
}
