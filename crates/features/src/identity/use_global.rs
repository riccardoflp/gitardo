//! `identity.use-global`: removes the identity the repository set for itself, so that the one of the global
//! configuration applies again.

use model::contract::{Answers, Context, Danger, IdentityKeys, Outcome, Plan, Refusal, Step, Text, Undo};

use super::read_identity;

/// The keys of the identity a repository may set for itself, in the order they are unset.
const KEYS: [&str; 3] = ["user.name", "user.email", "user.signingkey"];

/// Unsets each of `user.name`, `user.email` and `user.signingkey` that the repository has of its own, in that order:
/// `git config --unset-all <key>`.
///
/// Feature: identity.use-global
#[must_use]
pub fn use_global(context: &Context, answers: &Answers) -> Outcome {
    let keys = match read_identity(answers) {
        Ok(keys) => keys,
        Err(read) => return *read,
    };
    match unset_plan(context, keys) {
        Some(plan) => Outcome::Run(plan),
        None => Outcome::Refuse(Refusal::saying("This repository already uses the global identity.")),
    }
}

/// Whether the repository sets one of the keys of an identity for itself.
pub(super) fn has_own_identity(keys: &IdentityKeys) -> bool {
    KEYS.iter().any(|key| keys.local_keys.iter().any(|local| local == key))
}

/// The plan that unsets the keys of the identity the repository has of its own; none when it has none.
pub(super) fn unset_plan(context: &Context, keys: &IdentityKeys) -> Option<Plan> {
    let steps: Vec<Step> = KEYS
        .iter()
        .filter(|key| keys.local_keys.iter().any(|local| local == *key))
        .map(|key| Step::git(context.repository.folder.clone(), ["config", "--unset-all", key]))
        .collect();
    (!steps.is_empty()).then(|| Plan {
        title: Text::from("Use the global identity"),
        steps,
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Switching identity sets one again.")),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::use_global;
    use crate::identity::IDENTITY;
    use crate::testing::{context, lines, plan, state};
    use model::contract::{Answer, Answers, IdentityKeys, Outcome, Query, Target};

    fn with_keys(keys: &[&str]) -> Answers {
        Answers::new().with(
            IDENTITY,
            Answer::Identity(IdentityKeys {
                local_keys: keys.iter().map(|key| (*key).to_owned()).collect(),
                ..IdentityKeys::default()
            }),
        )
    }

    #[test]
    fn identity_use_global__unsets_the_name_and_email_of_the_repository() {
        let context = context(state(), Target::Head);
        assert_eq!(
            use_global(&context, &Answers::new()),
            Outcome::Read(IDENTITY, Query::Identity)
        );
        let plan = plan(use_global(&context, &with_keys(&["user.email", "user.name"])));
        assert_eq!(
            lines(&plan),
            ["git config --unset-all user.name", "git config --unset-all user.email"]
        );
    }

    #[test]
    fn identity_use_global__unsets_the_signing_key_too() {
        let plan = plan(use_global(
            &context(state(), Target::Head),
            &with_keys(&["user.name", "user.signingkey", "user.useconfigonly"]),
        ));
        assert_eq!(
            lines(&plan),
            [
                "git config --unset-all user.name",
                "git config --unset-all user.signingkey"
            ]
        );
    }

    #[test]
    fn refuses_a_repository_with_no_identity_of_its_own() {
        let Outcome::Refuse(refusal) = use_global(&context(state(), Target::Head), &with_keys(&[])) else {
            panic!("nothing to unset is refused")
        };
        assert_eq!(
            refusal.message.as_str(),
            "This repository already uses the global identity."
        );
    }
}
