//! `tag.create`: puts a tag on a commit, a lightweight one, or an annotated one with a message.

use model::contract::{
    AnswerKey, Answers, Context, Danger, FollowUp, Outcome, Plan, Question, QuestionKind, Step, Target, Text, Undo,
    Validation,
};

use model::tag::TagsOnRemote;

use crate::branch::target::refuse;

/// The name typed for the tag.
pub const NAME: AnswerKey = AnswerKey("name");

/// The message typed for the tag: empty for a lightweight one.
pub const MESSAGE: AnswerKey = AnswerKey("message");

/// Asks for the name of a tag and its message, and puts it on the commit it was invoked on: annotated with
/// `git tag -a` when the message has words, lightweight otherwise.
///
/// Feature: tag.create
#[must_use]
pub fn create(context: &Context, answers: &Answers) -> Outcome {
    let commit = match &context.target {
        Target::Commit(commit) => commit.clone(),
        _ => match &context.repository.head.commit {
            Some(commit) => commit.clone(),
            None => return refuse("There is no commit to tag yet."),
        },
    };
    let Some(name) = answers.text(NAME) else {
        return Outcome::Ask(name_question(context));
    };
    let Some(message) = answers.text(MESSAGE) else {
        return Outcome::Ask(message_question(name));
    };
    let message = message.trim();
    let arguments = if message.is_empty() {
        vec!["tag", name, commit.as_str()]
    } else {
        vec!["tag", "-a", name, "-m", message, commit.as_str()]
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Create tag {name} at {}", commit.short())),
        steps: vec![Step::git(context.repository.folder.clone(), arguments)],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "Not taken back by Undo, since nothing moved HEAD: delete the tag instead.",
        )),
        after: vec![FollowUp::TagsOnRemote(Box::new(TagsOnRemote::not_pushed(
            None,
            vec![name.to_owned()],
        )))],
        on_failure: Vec::new(),
    })
}

/// The input box of the name, which takes no name of a tag there is.
fn name_question(context: &Context) -> Question {
    let taken = context.repository.tags.iter().map(|tag| tag.name.clone()).collect();
    Question {
        key: NAME,
        kind: QuestionKind::Text {
            placeholder: Some(Text::from("v1.0.0")),
            validation: Validation::ReferenceName {
                taken,
                taken_message: Text::from("A tag with that name already exists"),
            },
        },
        title: Text::from("Create tag"),
        detail: None,
        default: None,
    }
}

/// The input box of the message, which may be left empty.
fn message_question(name: &str) -> Question {
    Question {
        key: MESSAGE,
        kind: QuestionKind::Text {
            placeholder: None,
            validation: Validation::Anything,
        },
        title: Text::from(format!("Create tag `{name}`")),
        detail: Some(Text::from(
            "Message for an annotated tag. Leave empty for a lightweight tag.",
        )),
        default: None,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{MESSAGE, NAME, create};
    use crate::testing::{commit, context, lines, plan, question, state, tag};
    use model::contract::{Answer, Answers, QuestionKind, Target};

    fn named(name: &str, message: &str) -> Answers {
        Answers::new()
            .with(NAME, Answer::Text(name.to_owned()))
            .with(MESSAGE, Answer::Text(message.to_owned()))
    }

    #[test]
    fn tag_create__an_empty_message_makes_a_lightweight_tag() {
        let context = context(state(), Target::Commit(commit('b')));
        let plan = plan(create(&context, &named("v1.0.0", "  ")));
        assert_eq!(lines(&plan), [format!("git tag v1.0.0 {}", commit('b'))]);
    }

    #[test]
    fn tag_create__a_message_makes_an_annotated_tag() {
        let context = context(state(), Target::Commit(commit('b')));
        let plan = plan(create(&context, &named("v1.0.0", " First release\n")));
        assert_eq!(
            lines(&plan),
            [format!("git tag -a v1.0.0 -m \"First release\" {}", commit('b'))]
        );
    }

    #[test]
    fn tag_create__asks_the_name_then_the_message() {
        let mut state = state();
        state.tags.push(tag("v0.9", 'a'));
        let context = context(state, Target::Commit(commit('b')));
        let name = question(create(&context, &Answers::new()));
        assert_eq!(name.title.as_str(), "Create tag");
        let QuestionKind::Text { validation, .. } = name.kind else {
            panic!("the name is typed")
        };
        assert_eq!(
            validation.problem("v0.9").map(|text| text.to_string()),
            Some(String::from("A tag with that name already exists"))
        );
        let message = question(create(
            &context,
            &Answers::new().with(NAME, Answer::Text(String::from("v1.0.0"))),
        ));
        assert_eq!(message.title.as_str(), "Create tag `v1.0.0`");
    }
}
