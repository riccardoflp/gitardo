//! The AI provider as the settings name it (`ai.provider`, `ai.endpoint`, `ai.model`, `ai.effort`), the models it
//! lists, its key, and what is asked of it.

use std::fmt;

/// `ai.provider`: the kind of HTTP interface of the language model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AiProvider {
    /// `anthropic`: the Messages API.
    Anthropic,
    /// `openai-compatible`: the chat completions of `OpenAI`, as many servers offer them.
    OpenAiCompatible,
    /// `copilot`: GitHub Copilot, through the GitHub Copilot CLI and the user's own sign-in (`ai.copilot`).
    Copilot,
}

impl AiProvider {
    /// The values the setting takes, in this order: `anthropic`, `openai-compatible`, `copilot`.
    pub const VALUES: [&'static str; 3] = ["anthropic", "openai-compatible", "copilot"];

    /// The provider a value of the setting names; none for an empty value or one it does not take.
    #[must_use]
    pub fn from_setting(value: &str) -> Option<AiProvider> {
        match value {
            "anthropic" => Some(AiProvider::Anthropic),
            "openai-compatible" => Some(AiProvider::OpenAiCompatible),
            "copilot" => Some(AiProvider::Copilot),
            _ => None,
        }
    }

    /// The value of the setting that names it.
    #[must_use]
    pub fn setting(self) -> &'static str {
        match self {
            AiProvider::Anthropic => "anthropic",
            AiProvider::OpenAiCompatible => "openai-compatible",
            AiProvider::Copilot => "copilot",
        }
    }

    /// The name its key is kept under in the credential store of the system: `ai.anthropic`.
    #[must_use]
    pub fn key_name(self) -> String {
        format!("ai.{}", self.setting())
    }
}

/// `ai.effort`: how much the model thinks before it answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Effort {
    /// `low`.
    Low,
    /// `medium`.
    Medium,
    /// `high`.
    High,
    /// `xhigh`.
    ExtraHigh,
    /// `max`.
    Max,
}

impl Effort {
    /// Every effort, from the least.
    pub const ALL: [Effort; 5] = [
        Effort::Low,
        Effort::Medium,
        Effort::High,
        Effort::ExtraHigh,
        Effort::Max,
    ];

    /// The values the setting takes besides empty, in this order.
    pub const VALUES: [&'static str; 5] = ["low", "medium", "high", "xhigh", "max"];

    /// The effort a value of the setting names; none for an empty value or one it does not take.
    #[must_use]
    pub fn from_setting(value: &str) -> Option<Effort> {
        Effort::ALL.into_iter().find(|effort| effort.setting() == value)
    }

    /// The value of the setting that names it.
    #[must_use]
    pub fn setting(self) -> &'static str {
        match self {
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
            Effort::ExtraHigh => "xhigh",
            Effort::Max => "max",
        }
    }

    /// The effort in words, in lower case: `high`, `extra high`.
    #[must_use]
    pub fn words(self) -> &'static str {
        match self {
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
            Effort::ExtraHigh => "extra high",
            Effort::Max => "max",
        }
    }
}

/// The settings of the AI provider.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AiSettings {
    /// `ai.provider`; none while it is empty, which is no AI.
    pub provider: Option<AiProvider>,
    /// `ai.endpoint`: the base URL of the provider; empty for the provider's own.
    pub endpoint: String,
    /// `ai.model`: the model asked; empty for the first model the provider lists.
    pub model: String,
    /// `ai.effort`; none for as the model is set up.
    pub effort: Option<Effort>,
}

/// A model the provider lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiModel {
    /// Its identifier, as `ai.model` names it.
    pub id: String,
    /// Its name for people, or its identifier when the provider gives none.
    pub name: String,
}

/// The key of the AI provider. It is never shown in `Debug`, so that it reaches no log.
#[derive(Clone, PartialEq, Eq)]
pub struct Key(pub String);

impl fmt::Debug for Key {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Key(<hidden>)")
    }
}

/// What listing the models of the provider came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiModels {
    /// The models, in the provider's order.
    Listed(Vec<AiModel>),
    /// The provider wants a key, and none is kept, or it refused the one it was given.
    KeyNeeded,
    /// It failed: why, in words.
    Failed(String),
    /// GitHub Copilot cannot be asked until the user does something first (`ai.copilot`).
    CopilotNeeds(CopilotNeed),
}

/// What the user does before GitHub Copilot can be asked (`ai.copilot`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopilotNeed {
    /// Install the GitHub Copilot CLI: `copilot` is not on the `PATH`.
    Install,
    /// Sign in to it, with `copilot login`.
    SignIn,
}

impl CopilotNeed {
    /// What it says: *The GitHub Copilot CLI is not installed. Install it, then try again.*
    #[must_use]
    pub fn sentence(self) -> &'static str {
        match self {
            CopilotNeed::Install => "The GitHub Copilot CLI is not installed. Install it, then try again.",
            CopilotNeed::SignIn => "GitHub Copilot is not signed in.",
        }
    }

    /// The words of the button that does it: **How to install** or **Sign in**.
    #[must_use]
    pub fn button(self) -> &'static str {
        match self {
            CopilotNeed::Install => "How to install",
            CopilotNeed::SignIn => "Sign in",
        }
    }
}

/// GitHub's page on installing the GitHub Copilot CLI, which **How to install** opens.
pub const COPILOT_INSTALL_PAGE: &str = "https://docs.github.com/copilot/how-tos/set-up/install-copilot-cli";

/// The command **Sign in** runs in a terminal of the bottom panel.
pub const COPILOT_SIGN_IN: &str = "copilot login";

/// What is asked of the language model: one message from the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiRequest {
    /// The text of the message.
    pub prompt: String,
}
