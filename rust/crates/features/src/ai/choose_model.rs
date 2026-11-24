//! `ai.choose-model`: chooses the model of the AI provider that is asked about conflicts, and how much it thinks,
//! written in the user's settings file.

use model::ai::{AiModel, AiModels, CopilotNeed, Effort, Key};
use model::contract::{
    Answer, AnswerKey, Answers, Choice, Context, Danger, FeatureId, FollowUp, Outcome, Plan, Query, Question,
    QuestionKind, Refusal, SettingWritten, Step, Text, Undo, Validation,
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

/// **How to install**, offered when the GitHub Copilot CLI is not on the `PATH`: the window opens GitHub's page on
/// installing it (`ai.copilot`).
pub const COPILOT_INSTALL: FeatureId = FeatureId("ai.copilot-install");
/// **Sign in**, offered when the GitHub Copilot CLI is not signed in: the window runs `copilot login` in a terminal of
/// the bottom panel (`ai.copilot`).
pub const COPILOT_SIGN_IN: FeatureId = FeatureId("ai.copilot");

/// The prefix of the choice of a model listed.
const MODEL_PREFIX: &str = "model:";

/// Lists the models of the provider, asking for its key when it wants one; asks which one resolves conflicts, then
/// how much it thinks; and writes `ai.model` and `ai.effort` in the user's settings file, removing each for the first
/// model and for as the model is set up. Under a host that has its own models (`uses_host_models`), they are the
/// host's, no provider or key is needed, and the host writes the two settings (`FollowUp::WriteSetting`).
///
/// Feature: ai.choose-model
#[must_use]
pub fn choose_model(context: &Context, answers: &Answers) -> Outcome {
    let settings = &context.settings.ai;
    let hosted = context.settings.uses_host_models;
    if let Some(refused) = refused_first(context) {
        return Outcome::Refuse(refused);
    }
    let file = context.settings.user_settings_file.clone();
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
        return Outcome::Ask(model_question(&models, first, &settings.model, hosted));
    };
    let Some(effort) = answers.choice(EFFORT) else {
        return Outcome::Ask(effort_question(settings.effort));
    };
    let model = model.strip_prefix(MODEL_PREFIX);
    let effort = Effort::from_setting(effort);
    let name = model_name(&models, first, model);
    // Under a host, the picks are all there is to agree to: the host writes the two settings, and nothing more is
    // shown, under `commandPreview` `all` too (G6).
    let danger = if hosted { Danger::Agreed } else { Danger::None };
    let plan = |steps: Vec<Step>, after: Vec<FollowUp>| {
        Outcome::Run(Plan {
            title: Text::from(format!("Resolve conflicts with {name}")),
            steps,
            danger,
            undo: Undo::Impossible(Text::from("Choosing again sets them back.")),
            after,
            on_failure: Vec::new(),
        })
    };
    let Some(file) = file.filter(|_| !hosted) else {
        return plan(Vec::new(), settings_written(model, effort));
    };
    let Some(files) = answers.files(SETTINGS) else {
        return Outcome::Read(SETTINGS, Query::FileTexts { paths: vec![file] });
    };
    let Some(written) = written(files, model, effort) else {
        return Outcome::Refuse(Refusal::saying(
            "The user's settings file is not JSON with comments, so the model cannot be written in it.",
        ));
    };
    let keys = settings
        .provider
        .zip(typed)
        .map(|(provider, key)| FollowUp::KeepAiKey(provider, key));
    plan(
        vec![Step::Write {
            path: file,
            contents: written.into_bytes(),
        }],
        keys.into_iter().collect(),
    )
}

/// Why the models cannot be chosen at all: no AI provider is set up, or no settings file of the user can be written;
/// under a host that has its own models, neither is needed.
fn refused_first(context: &Context) -> Option<Refusal> {
    if context.settings.uses_host_models {
        return None;
    }
    if context.settings.ai.provider.is_none() {
        return Some(Refusal {
            message: Text::from("No AI provider is set up."),
            offer: Some((FeatureId("settings.edit"), Text::from("Open settings"))),
        });
    }
    context
        .settings
        .user_settings_file
        .is_none()
        .then(|| Refusal::saying("Gitardo has no settings file of the user to write the model in."))
}

/// The name of the model chosen, as its provider lists it: the first's for the first model, the id of one not listed.
fn model_name<'a>(models: &'a [AiModel], first: &'a AiModel, chosen: Option<&'a str>) -> &'a str {
    chosen.map_or(first.name.as_str(), |id| {
        models
            .iter()
            .find(|listed| listed.id == id)
            .map_or(id, |listed| listed.name.as_str())
    })
}

/// The two settings the host writes: `ai.model`, removed for the first model, and `ai.effort`, removed for as the model
/// is set up.
fn settings_written(model: Option<&str>, effort: Option<Effort>) -> Vec<FollowUp> {
    vec![
        setting_written("ai.model", model),
        setting_written("ai.effort", effort.map(Effort::setting)),
    ]
}

/// A setting for the host to write, its value as a JSON string, or removed.
fn setting_written(key: &str, value: Option<&str>) -> FollowUp {
    FollowUp::WriteSetting(Box::new(SettingWritten {
        key: key.to_owned(),
        value: value.map(|value| json::to_compact_text(&json::Value::String(value.to_owned()))),
    }))
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
        AiModels::CopilotNeeds(need) => Err(Box::new(Outcome::Refuse(copilot_refusal(*need)))),
    }
}

/// What GitHub Copilot needs first, with the button that does it: **How to install** or **Sign in** (`ai.copilot`).
#[must_use]
pub fn copilot_refusal(need: CopilotNeed) -> Refusal {
    let feature = match need {
        CopilotNeed::Install | CopilotNeed::Update => COPILOT_INSTALL,
        CopilotNeed::SignIn => COPILOT_SIGN_IN,
    };
    Refusal {
        message: Text::from(need.sentence()),
        offer: Some((feature, Text::from(need.button()))),
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

/// **The model that resolves conflicts**: the first model of the provider, or **The first model** under a host whose
/// models they are, then each model listed; the current choice is the default.
fn model_question(models: &[AiModel], first: &AiModel, current: &str, hosted: bool) -> Question {
    let first_label = if hosted {
        "The first model"
    } else {
        "The first model of the provider"
    };
    let mut choices = vec![choice(String::from(FIRST_MODEL), first_label, Some(&first.name))];
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
