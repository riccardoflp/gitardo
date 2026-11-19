//! `tag.delete-remote`: deletes a tag on a remote, for everyone, and here too when asked.

use model::contract::{
    Answer, AnswerKey, Answers, Choice, Context, Danger, FollowUp, Outcome, Plan, Question, QuestionKind, Step, Text,
    Undo,
};

use model::tag::TagsOnRemote;

use crate::branch::target::{self, refuse};

/// Whether the tag is deleted here too, after the remote: **Delete here and from `<remote>`**.
pub const HERE: AnswerKey = AnswerKey("here");

/// The remote the tag is deleted from, when the repository has several.
pub const REMOTE: AnswerKey = AnswerKey("remote");

/// Deletes the tag it was invoked on from a remote with `git push <remote> --delete refs/tags/<name>`, the tag named in
/// full so that a branch of its name there is left alone; with **here and from**, then `git tag -d`, which does not run
/// when the push fails. Asks which remote when the repository has several, the one of `remote.default` first.
///
/// Feature: tag.delete-remote
#[must_use]
pub fn delete_remote(context: &Context, answers: &Answers) -> Outcome {
    let tag = match target::tag(context) {
        Ok(tag) => tag,
        Err(refusal) => return *refusal,
    };
    let remotes = &context.repository.remotes;
    let Some(default) = model::remote::default(remotes) else {
        return refuse("This repository has no remote.");
    };
    let name = tag.name.as_str();
    let remote = match (remotes.len(), answers.choice(REMOTE)) {
        (_, Some(remote)) => remote.to_owned(),
        (1, None) => default,
        (_, None) => return Outcome::Ask(remote_question(name, remotes, &default)),
    };
    let is_here_too = answers.flag(HERE).unwrap_or(false);
    let folder = context.repository.folder.clone();
    let mut steps = vec![Step::git(
        folder.clone(),
        ["push", remote.as_str(), "--delete", &format!("refs/tags/{name}")],
    )];
    let mut warning = format!(
        "This deletes the tag {name} on {remote} for everyone. Whoever fetched it keeps it, and may push it again."
    );
    let title = if is_here_too {
        steps.push(Step::git(folder, ["tag", "-d", name]));
        let here = format!(
            " The tag, on {}, is deleted from this repository too, with its message if it has one.",
            tag.commit.short()
        );
        warning.push_str(&here);
        format!("Delete tag {name} here and from {remote}")
    } else {
        format!("Delete tag {name} from {remote}")
    };
    Outcome::Run(Plan {
        title: Text::from(title),
        steps,
        danger: Danger::Destructive(Text::from(warning)),
        undo: Undo::Impossible(Text::from(
            "A push changes the remote, which Undo does not take back. While the tag is still here, Push tag sends it \
             again.",
        )),
        after: vec![FollowUp::TagsOnRemote(Box::new(TagsOnRemote::not_pushed(
            Some(&remote),
            vec![name.to_owned()],
        )))],
        on_failure: Vec::new(),
    })
}

/// The pick of the remote, the one of `remote.default` first.
fn remote_question(name: &str, remotes: &[String], default: &str) -> Question {
    let mut ordered: Vec<&String> = remotes.iter().collect();
    ordered.sort_by_key(|remote| remote.as_str() != default);
    Question {
        key: REMOTE,
        kind: QuestionKind::Pick {
            choices: ordered
                .into_iter()
                .map(|remote| Choice {
                    identifier: remote.clone(),
                    label: Text::from(remote.as_str()),
                    description: None,
                })
                .collect(),
        },
        title: Text::from(format!("Delete `{name}` from")),
        detail: None,
        default: Some(Answer::Choice(default.to_owned())),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{HERE, REMOTE, delete_remote};
    use crate::testing::{context, lines, plan, question, state, tag};
    use model::contract::{Answer, Answers, Danger, Outcome, QuestionKind, Target};

    fn on_v1(remotes: &[&str]) -> model::contract::Context {
        let mut state = state();
        state.tags.push(tag("v1", 'b'));
        state.remotes = remotes.iter().map(|remote| (*remote).to_owned()).collect();
        context(state, Target::Tag(String::from("v1")))
    }

    #[test]
    fn tag_delete_remote__delete_from_origin_warns_and_pushes_the_deletion_of_the_tag_named_in_full() {
        let outcome = delete_remote(&on_v1(&["origin"]), &Answers::new());
        let plan = plan(outcome);
        assert!(matches!(plan.danger, Danger::Destructive(_)));
        assert_eq!(lines(&plan), ["git push origin --delete refs/tags/v1"]);
    }

    #[test]
    fn tag_delete_remote__here_and_from_origin_pushes_then_deletes_the_tag_here() {
        let answers = Answers::new().with(HERE, Answer::Flag(true));
        assert_eq!(
            lines(&plan(delete_remote(&on_v1(&["origin"]), &answers))),
            ["git push origin --delete refs/tags/v1", "git tag -d v1"]
        );
    }

    #[test]
    fn tag_delete_remote__with_several_remotes_it_asks_which_the_default_first() {
        let asked = question(delete_remote(&on_v1(&["upstream", "origin"]), &Answers::new()));
        assert_eq!(asked.title.as_str(), "Delete `v1` from");
        let QuestionKind::Pick { choices } = asked.kind else {
            panic!("the remote is picked")
        };
        assert_eq!(choices[0].identifier, "origin");
        let answers = Answers::new().with(REMOTE, Answer::Choice(String::from("upstream")));
        assert_eq!(
            lines(&plan(delete_remote(&on_v1(&["upstream", "origin"]), &answers))),
            ["git push upstream --delete refs/tags/v1"]
        );
    }

    #[test]
    fn tag_delete_remote__a_repository_with_no_remote_refuses() {
        assert!(matches!(
            delete_remote(&on_v1(&[]), &Answers::new()),
            Outcome::Refuse(_)
        ));
    }
}
