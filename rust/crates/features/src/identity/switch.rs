//! `identity.switch`: sets the name and email that commits are made with in this repository, from a profile of the
//! settings or typed.

use model::contract::{
    Answer, AnswerKey, Answers, Choice, Context, Danger, IdentityKeys, IdentityProfile, Outcome, Plan, Question,
    QuestionKind, Refusal, Step, Text, Undo, Validation,
};

use super::read_identity;
use super::use_global::{has_own_identity, unset_plan};

/// The identity picked.
pub const PICK: AnswerKey = AnswerKey("identity-pick");
/// The name typed.
pub const NAME: AnswerKey = AnswerKey("name");
/// The email typed.
pub const EMAIL: AnswerKey = AnswerKey("email");

/// The choice that asks for a name and an email.
const ENTER: &str = "enter";
/// The choice that goes back to the global identity (`identity.use-global`).
const GLOBAL: &str = "global";
/// The start of the choice of a profile, followed by its place in `identities`.
const PROFILE: &str = "profile:";

/// Asks which identity to commit as, then sets it in the configuration of the repository only: `git config user.name`,
/// `git config user.email`, and `git config user.signingkey` for a profile that has a key; for one that has none,
/// when the repository has a key of its own, `git config --unset-all user.signingkey`, so that commits are not signed
/// as someone else. **Use the global identity** runs `identity.use-global`.
///
/// Feature: identity.switch
#[must_use]
pub fn switch(context: &Context, answers: &Answers) -> Outcome {
    let keys = match read_identity(answers) {
        Ok(keys) => keys,
        Err(read) => return *read,
    };
    let Some(picked) = answers.choice(PICK) else {
        return Outcome::Ask(pick_question(context, keys));
    };
    let chosen = if picked == GLOBAL {
        return unset_plan(context, keys).map_or_else(
            || Outcome::Refuse(Refusal::saying("This repository already uses the global identity.")),
            Outcome::Run,
        );
    } else if picked == ENTER {
        match typed(keys, answers) {
            Ok(profile) => profile,
            Err(outcome) => return *outcome,
        }
    } else {
        let profile = picked
            .strip_prefix(PROFILE)
            .and_then(|place| place.parse::<usize>().ok())
            .and_then(|place| context.settings.identities.get(place));
        match profile {
            Some(profile) => profile.clone(),
            None => return Outcome::Refuse(Refusal::saying("That identity is not in the settings any more.")),
        }
    };
    set_plan(context, keys, &chosen)
}

/// The pick of the identity: each profile of `identities`, **Enter a name and email…**, and **Use the global identity**
/// when the repository has an identity of its own; the identity used now is said under the title.
fn pick_question(context: &Context, keys: &IdentityKeys) -> Question {
    let now = &keys.identity;
    let mut choices: Vec<Choice> = context
        .settings
        .identities
        .iter()
        .enumerate()
        .map(|(place, profile)| {
            let is_in_use =
                now.name.as_deref() == Some(profile.name.as_str()) && now.email.as_deref() == Some(&profile.email);
            Choice {
                identifier: format!("{PROFILE}{place}"),
                label: Text::from(format!("{} <{}>", profile.name, profile.email)),
                description: is_in_use.then(|| Text::from("In use")),
            }
        })
        .collect();
    choices.push(Choice {
        identifier: ENTER.to_owned(),
        label: Text::from("Enter a name and email…"),
        description: None,
    });
    if has_own_identity(keys) {
        choices.push(Choice {
            identifier: GLOBAL.to_owned(),
            label: Text::from("Use the global identity"),
            description: Some(Text::from("Remove this repository's own setting")),
        });
    }
    let repository = context
        .repository
        .folder
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    let detail = match (&now.name, &now.email) {
        (Some(name), Some(email)) => format!("Now: {name} <{email}>"),
        _ => String::from("No identity is configured"),
    };
    Question {
        key: PICK,
        kind: QuestionKind::Pick { choices },
        title: Text::from(format!("Commit in {repository} as")),
        detail: Some(Text::from(detail)),
        default: None,
    }
}

/// The name, then the email, typed: each starts from the one used now.
fn typed(keys: &IdentityKeys, answers: &Answers) -> Result<IdentityProfile, Box<Outcome>> {
    let asked = |key: AnswerKey, title: &str, now: &Option<String>| Question {
        key,
        kind: QuestionKind::Text {
            placeholder: None,
            validation: Validation::Anything,
        },
        title: Text::from(title),
        detail: None,
        default: Some(Answer::Text(now.clone().unwrap_or_default())),
    };
    let name = answers
        .text(NAME)
        .ok_or_else(|| Box::new(Outcome::Ask(asked(NAME, "Name", &keys.identity.name))))?;
    let email = answers
        .text(EMAIL)
        .ok_or_else(|| Box::new(Outcome::Ask(asked(EMAIL, "Email", &keys.identity.email))))?;
    if name.trim().is_empty() || email.trim().is_empty() {
        return Err(Box::new(Outcome::Refuse(Refusal::saying(
            "A name and an email are both needed: nothing was changed.",
        ))));
    }
    Ok(IdentityProfile {
        name: name.to_owned(),
        email: email.to_owned(),
        signing_key: None,
    })
}

/// The plan that sets an identity, the name and email without the spaces around them; refused when a value starts
/// with `-`, which git would read as an option.
fn set_plan(context: &Context, keys: &IdentityKeys, chosen: &IdentityProfile) -> Outcome {
    let name = chosen.name.trim();
    let email = chosen.email.trim();
    let key = chosen.signing_key.as_deref().map(str::trim);
    if let Some(value) = [Some(name), Some(email), key]
        .into_iter()
        .flatten()
        .find(|value| value.starts_with('-'))
    {
        return Outcome::Refuse(Refusal::saying(&format!("{value} is not a valid name.")));
    }
    let folder = &context.repository.folder;
    let mut steps = vec![
        Step::git(folder.clone(), ["config", "user.name", name]),
        Step::git(folder.clone(), ["config", "user.email", email]),
    ];
    match key {
        Some(key) => steps.push(Step::git(folder.clone(), ["config", "user.signingkey", key])),
        None if keys.local_keys.iter().any(|local| local == "user.signingkey") => {
            steps.push(Step::git(folder.clone(), ["config", "--unset-all", "user.signingkey"]));
        }
        None => {}
    }
    Outcome::Run(Plan {
        title: Text::from(format!("Commit as {name} <{email}> in this repository")),
        steps,
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "Switching again, or using the global identity, takes it back.",
        )),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
