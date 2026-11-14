//! `ai.choose-model`: chooses the model of the AI provider that is asked about conflicts, and how much it thinks,
//! written in the user's settings file.

use model::ai::{AiModel, AiModels, Effort, Key};
use model::contract::{
    Answer, AnswerKey, Answers, Choice, Context, Danger, FeatureId, FollowUp, Outcome, Plan, Query, Question,
    QuestionKind, Refusal, Step, Text, Undo, Validation,
};

/// The models listed with the key kept.
pub const MODELS: AnswerKey = AnswerKey("models");
/// The key typed, when the provider wants one and none is kept.
pub const KEY: AnswerKey = AnswerKey("key");
/// The models listed with the key typed.
pub const MODELS_WITH_KEY: AnswerKey = AnswerKey("models with key");
/// The model chosen.
pub const MODEL: AnswerKey = AnswerKey("model");
/// The effort chosen.
pub const EFFORT: AnswerKey = AnswerKey("effort");
/// The user's settings file, read before it is written.
pub const SETTINGS: AnswerKey = AnswerKey("settings file");

/// The choice of the first model of the provider, which removes `ai.model`.
pub const FIRST_MODEL: &str = "first";
/// The choice of the effort the model is set up with, which removes `ai.effort`.
pub const AS_SET_UP: &str = "as set up";

/// The prefix of the choice of a model listed.
const MODEL_PREFIX: &str = "model:";

/// Lists the models of the provider, asking for its key when it wants one; asks which one resolves conflicts, then
/// how much it thinks; and writes `ai.model` and `ai.effort` in the user's settings file, removing each for the first
/// model and for as the model is set up.
///
/// Feature: ai.choose-model
#[must_use]
pub fn choose_model(context: &Context, answers: &Answers) -> Outcome {
    let settings = &context.settings.ai;
    let Some(provider) = settings.provider else {
        return Outcome::Refuse(Refusal {
            message: Text::from("No AI provider is set up."),
            offer: Some((FeatureId("settings.edit"), Text::from("Open settings"))),
        });
    };
    let Some(file) = context.settings.user_settings_file.clone() else {
        return Outcome::Refuse(Refusal::saying(
            "Gitardo has no settings file of the user to write the model in.",
        ));
    };
    let typed = answers.text(KEY).map(|key| Key(key.trim().to_owned()));
    let models = match listed(context, answers, typed.as_ref()) {
        Ok(models) => models,
        Err(outcome) => return *outcome,
    };
    let Some(first) = models.first() else {
        return Outcome::Refuse(Refusal::saying(
            "No language model is available from the AI provider. Set one up in the settings.",
        ));
    };
    let Some(model) = answers.choice(MODEL) else {
        return Outcome::Ask(model_question(&models, first, &settings.model));
    };
    let Some(effort) = answers.choice(EFFORT) else {
        return Outcome::Ask(effort_question(settings.effort));
    };
    let Some(files) = answers.files(SETTINGS) else {
        return Outcome::Read(SETTINGS, Query::FileTexts { paths: vec![file] });
    };
    let model = model.strip_prefix(MODEL_PREFIX);
    let Some(written) = written(files, model, Effort::from_setting(effort)) else {
        return Outcome::Refuse(Refusal::saying(
            "The user's settings file is not JSON with comments, so the model cannot be written in it.",
        ));
    };
    let name = model.map_or(first.name.as_str(), |id| {
        models
            .iter()
            .find(|listed| listed.id == id)
            .map_or(id, |listed| listed.name.as_str())
    });
    Outcome::Run(Plan {
        title: Text::from(format!("Resolve conflicts with {name}")),
        steps: vec![Step::Write {
            path: file,
            contents: written.into_bytes(),
        }],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Choosing again sets them back.")),
        after: typed
            .into_iter()
            .map(|key| FollowUp::KeepAiKey(provider, key))
            .collect(),
        on_failure: Vec::new(),
    })
}

/// The models listed, with the key kept or, when the provider wants one, with the key typed; or what to do first.
fn listed(context: &Context, answers: &Answers, typed: Option<&Key>) -> Result<Vec<AiModel>, Box<Outcome>> {
    let read = |key: AnswerKey, given: Option<&Key>| {
        Box::new(Outcome::Read(
            key,
            Query::AiModels {
                settings: (*context.settings.ai).clone(),
                key: given.cloned(),
            },
        ))
    };
    let Some(Answer::AiModels(kept)) = answers.get(MODELS) else {
        return Err(read(MODELS, None));
    };
    let models = match (kept, typed) {
        (AiModels::KeyNeeded, None) => return Err(Box::new(Outcome::Ask(key_question()))),
        (AiModels::KeyNeeded, Some(typed)) => match answers.get(MODELS_WITH_KEY) {
            Some(Answer::AiModels(models)) => models,
            _ => return Err(read(MODELS_WITH_KEY, Some(typed))),
        },
        (models, _) => models,
    };
    match models {
        AiModels::Listed(models) => Ok(models.clone()),
        AiModels::KeyNeeded => Err(Box::new(Outcome::Refuse(Refusal::saying(
            "The AI provider refused the key.",
        )))),
        AiModels::Failed(reason) => Err(Box::new(Outcome::Refuse(Refusal::saying(reason)))),
    }
}

/// The question that asks for the key of the provider, in a box that hides what is typed.
fn key_question() -> Question {
    Question {
        key: KEY,
        kind: QuestionKind::Secret {
            validation: Validation::Required(Text::from("Type the key")),
        },
        title: Text::from("Key of the AI provider"),
        detail: Some(Text::from(
            "Kept in the credential store of the system, and sent only to the AI provider",
        )),
        default: None,
    }
}

/// A choice of a pick.
fn choice(identifier: String, label: &str, description: Option<&str>) -> Choice {
    Choice {
        identifier,
        label: Text::from(label),
        description: description.map(Text::from),
    }
}

/// **The model that resolves conflicts**: the first model of the provider, then each model listed; the current choice
/// is the default.
fn model_question(models: &[AiModel], first: &AiModel, current: &str) -> Question {
    let mut choices = vec![choice(
        String::from(FIRST_MODEL),
        "The first model of the provider",
        Some(&first.name),
    )];
    choices.extend(models.iter().map(|model| {
        let description = (model.name != model.id).then_some(model.id.as_str());
        choice(format!("{MODEL_PREFIX}{}", model.id), &model.name, description)
    }));
    let chosen = if current.is_empty() {
        String::from(FIRST_MODEL)
    } else {
        format!("{MODEL_PREFIX}{current}")
    };
    Question {
        key: MODEL,
        kind: QuestionKind::Pick { choices },
        title: Text::from("The model that resolves conflicts"),
        detail: Some(Text::from("Gitardo asks it about the conflicts the rules leave")),
        default: Some(Answer::Choice(chosen)),
    }
}

/// **How much the model thinks before it answers**: as the model is set up, or one of the efforts; the current choice
/// is the default.
fn effort_question(current: Option<Effort>) -> Question {
    let mut choices = vec![choice(String::from(AS_SET_UP), "As the model is set up", None)];
    let labels = ["Low", "Medium", "High", "Extra high", "Max"];
    choices.extend(
        Effort::ALL
            .into_iter()
            .zip(labels)
            .map(|(effort, label)| choice(effort.setting().to_owned(), label, None)),
    );
    Question {
        key: EFFORT,
        kind: QuestionKind::Pick { choices },
        title: Text::from("How much the model thinks before it answers"),
        detail: Some(Text::from(
            "More is slower, and often better on hard conflicts. A model without such a setting ignores it.",
        )),
        default: Some(Answer::Choice(String::from(current.map_or(AS_SET_UP, Effort::setting)))),
    }
}

/// The settings file read with `ai.model` and `ai.effort` set, or removed for none; none when it is not JSON with
/// comments.
fn written(files: &[Option<Vec<u8>>], model: Option<&str>, effort: Option<Effort>) -> Option<String> {
    let text = files
        .first()
        .and_then(Option::as_deref)
        .map_or_default(String::from_utf8_lossy);
    let set = |text: &str, key: &str, value: Option<&str>| {
        let path = [json::Key::Member(key.to_owned())];
        match value {
            Some(value) => json::set_value(text, &path, &json::Value::String(value.to_owned())),
            None => json::remove_value(text, &path),
        }
    };
    let text = set(&text, "ai.model", model).ok()?;
    set(&text, "ai.effort", effort.map(Effort::setting)).ok()
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
