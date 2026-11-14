use super::{EMAIL, NAME, PICK, switch};
use crate::identity::IDENTITY;
use crate::testing::{context, lines, plan, question, state};
use model::contract::{Answer, Answers, Context, IdentityKeys, IdentityProfile, Outcome, QuestionKind, Target};
use model::state::Identity;

fn ada(signing_key: Option<&str>) -> IdentityProfile {
    IdentityProfile {
        name: String::from("Ada"),
        email: String::from("ada@home.example"),
        signing_key: signing_key.map(str::to_owned),
    }
}

fn with_profiles(profiles: Vec<IdentityProfile>) -> Context {
    let mut context = context(state(), Target::Head);
    context.settings.identities = profiles;
    context
}

fn read(local_keys: &[&str]) -> Answers {
    Answers::new().with(
        IDENTITY,
        Answer::Identity(IdentityKeys {
            identity: Identity {
                name: Some(String::from("Grace")),
                email: Some(String::from("grace@work.example")),
            },
            local_keys: local_keys.iter().map(|key| (*key).to_owned()).collect(),
        }),
    )
}

fn picked(answers: Answers, choice: &str) -> Answers {
    answers.with(PICK, Answer::Choice(choice.to_owned()))
}

#[test]
fn identity_switch__a_profile_without_a_key_unsets_the_key_of_the_repository() {
    let context = with_profiles(vec![ada(None)]);
    let plan = plan(switch(&context, &picked(read(&["user.signingkey"]), "profile:0")));
    assert_eq!(
        lines(&plan),
        [
            "git config user.name Ada",
            "git config user.email ada@home.example",
            "git config --unset-all user.signingkey"
        ]
    );
}

#[test]
fn identity_switch__a_profile_with_a_key_sets_it_third() {
    let context = with_profiles(vec![ada(Some("ABCD1234"))]);
    let plan = plan(switch(&context, &picked(read(&[]), "profile:0")));
    assert_eq!(
        lines(&plan).get(2).map(String::as_str),
        Some("git config user.signingkey ABCD1234")
    );
}

#[test]
fn identity_switch__without_a_key_of_the_repository_a_profile_with_none_runs_two_commands() {
    let context = with_profiles(vec![ada(None)]);
    let plan = plan(switch(&context, &picked(read(&["user.name"]), "profile:0")));
    assert_eq!(lines(&plan).len(), 2);
}

#[test]
fn identity_switch__never_writes_the_global_configuration() {
    let context = with_profiles(vec![ada(Some("KEY")), ada(None)]);
    let mut all = Vec::new();
    for (choice, keys) in [("profile:0", &["user.signingkey"][..]), ("profile:1", &[][..])] {
        all.extend(lines(&plan(switch(&context, &picked(read(keys), choice)))));
    }
    let typed = picked(read(&["user.signingkey"]), "enter")
        .with(NAME, Answer::Text(String::from("  Lin  ")))
        .with(EMAIL, Answer::Text(String::from("lin@example.com")));
    let typed_lines = lines(&plan(switch(&context, &typed)));
    assert_eq!(
        typed_lines.first().map(String::as_str),
        Some("git config user.name Lin")
    );
    all.extend(typed_lines);
    assert!(all.iter().all(|line| !line.contains("--global")), "{all:?}");
}

#[test]
fn identity_switch__use_the_global_identity_is_offered_only_to_a_repository_with_its_own() {
    let context = with_profiles(vec![ada(None)]);
    let choices = |answers: &Answers| match question(switch(&context, answers)).kind {
        QuestionKind::Pick { choices } => choices
            .into_iter()
            .map(|choice| choice.identifier)
            .collect::<Vec<String>>(),
        other => panic!("a pick, not {other:?}"),
    };
    assert_eq!(choices(&read(&[])), ["profile:0", "enter"]);
    assert_eq!(choices(&read(&["user.email"])), ["profile:0", "enter", "global"]);
}

#[test]
fn reads_first_says_what_is_used_now_and_refuses_a_value_that_is_an_option() {
    let context = with_profiles(Vec::new());
    assert!(matches!(switch(&context, &Answers::new()), Outcome::Read(..)));
    let asked = question(switch(&context, &read(&[])));
    assert_eq!(asked.title.as_str(), "Commit in repository as");
    assert_eq!(
        asked.detail.map(|detail| detail.as_str().to_owned()).as_deref(),
        Some("Now: Grace <grace@work.example>")
    );
    let typed = picked(read(&[]), "enter")
        .with(NAME, Answer::Text(String::from("-n")))
        .with(EMAIL, Answer::Text(String::from("x@y")));
    let Outcome::Refuse(refusal) = switch(&context, &typed) else {
        panic!("an option is refused")
    };
    assert_eq!(refusal.message.as_str(), "-n is not a valid name.");
}
