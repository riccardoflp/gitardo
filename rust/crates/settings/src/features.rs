//! What the operations read of the settings: [`FeatureSettings`], the size of a page of the timeline and the order of
//! the branches. The window and the engine read them the same way.

use json::Value;
use model::ai::{AiProvider, AiSettings, Effort};
use model::contract::{BlameSettings, CommandPreview, FeatureSettings, OpenIn, WorktreeOpenIn, WorktreeSettings};
use model::graph::PageSize;
use model::refs::SortOrder;

use crate::keys::{self, Scope};
use crate::{Lookup, ProblemReason};

/// `graph.pageSize`: how many commits the timeline reads at a time; the default when it is not set.
pub fn page_size(lookup: &mut Lookup) -> PageSize {
    lookup
        .number(keys::PAGE_SIZE, Scope::UserAndRepository)
        .map_or(PageSize::DEFAULT, |size| PageSize::from_setting(Some(size)))
}

/// `branches.sortOrder`: the order of the branches, the remote branches and the tags; none while it is not set.
pub fn sort_order(lookup: &mut Lookup) -> Option<SortOrder> {
    lookup.choice(keys::SORT_ORDER, &keys::SORT_ORDERS).map(|order| {
        if order == "date" {
            SortOrder::Date
        } else {
            SortOrder::Name
        }
    })
}

/// The settings the operations read: `commandPreview`, `branches.format`, `branches.inputs`, `branches.openIn`,
/// `submodules.updateOnCheckout`, `identities`, `pull.rebase`, `pull.autostash`, `cherryPick.recordOrigin`,
/// `conflicts.autoResolve`, `conflicts.ai`, `editor.external`, `blame.enabled`, `blame.ignoreWhitespaceAndMoves` and
/// `fileBlame.everyFile`.
pub fn feature_settings(lookup: &mut Lookup) -> FeatureSettings {
    let defaults = FeatureSettings::default();
    let command_preview = match lookup.choice(keys::COMMAND_PREVIEW, &keys::COMMAND_PREVIEWS) {
        Some("all") => CommandPreview::All,
        _ => CommandPreview::Off,
    };
    FeatureSettings {
        command_preview,
        update_submodules_on_checkout: lookup
            .boolean(keys::UPDATE_SUBMODULES_ON_CHECKOUT, Scope::UserAndRepository)
            .unwrap_or(defaults.update_submodules_on_checkout),
        branch_format: lookup
            .string(keys::BRANCH_FORMAT, Scope::UserAndRepository)
            .unwrap_or(defaults.branch_format),
        branch_inputs: lookup
            .first(
                (keys::BRANCH_INPUTS, Scope::UserAndRepository),
                crate::branch_inputs::branch_inputs,
            )
            .unwrap_or(defaults.branch_inputs),
        branch_open_in: match lookup.choice(keys::BRANCH_OPEN_IN, &keys::BRANCH_OPEN_INS) {
            Some("worktree") => OpenIn::Worktree,
            _ => OpenIn::Checkout,
        },
        pull_rebase: lookup
            .boolean(keys::PULL_REBASE, Scope::UserAndRepository)
            .unwrap_or(defaults.pull_rebase),
        pull_autostash: lookup
            .boolean(keys::PULL_AUTOSTASH, Scope::UserAndRepository)
            .unwrap_or(defaults.pull_autostash),
        identities: lookup
            .first((keys::IDENTITIES, Scope::User), identity_profiles)
            .unwrap_or(defaults.identities),
        cherry_pick_record_origin: lookup
            .boolean(keys::CHERRY_PICK_RECORD_ORIGIN, Scope::UserAndRepository)
            .unwrap_or(defaults.cherry_pick_record_origin),
        conflicts_auto_resolve: lookup
            .boolean(keys::CONFLICTS_AUTO_RESOLVE, Scope::User)
            .unwrap_or(defaults.conflicts_auto_resolve),
        conflicts_ai: lookup
            .boolean(keys::CONFLICTS_AI, Scope::User)
            .unwrap_or(defaults.conflicts_ai),
        external_editor: lookup
            .string(keys::EXTERNAL_EDITOR, Scope::User)
            .unwrap_or(defaults.external_editor),
        ai: Box::new(ai_settings(lookup)),
        user_settings_file: defaults.user_settings_file,
        trust: defaults.trust,
        blame: blame_settings(lookup, &defaults.blame),
        worktrees: worktree_settings(lookup),
        // The values given last for the inputs and the offset of the local time are the window's, set when an
        // operation starts.
        ..defaults
    }
}

/// `blame.enabled`, `blame.ignoreWhitespaceAndMoves` and `fileBlame.everyFile`, from the user's file.
fn blame_settings(lookup: &mut Lookup, defaults: &BlameSettings) -> BlameSettings {
    BlameSettings {
        is_enabled: lookup
            .boolean(keys::BLAME_ENABLED, Scope::User)
            .unwrap_or(defaults.is_enabled),
        ignores_whitespace: lookup
            .boolean(keys::BLAME_IGNORE_WHITESPACE, Scope::User)
            .unwrap_or(defaults.ignores_whitespace),
        is_on_every_file: lookup
            .boolean(keys::FILE_BLAME_EVERY_FILE, Scope::User)
            .unwrap_or(defaults.is_on_every_file),
    }
}

/// The AI provider: `ai.provider`, `ai.endpoint`, `ai.model` and `ai.effort`, from the user's file; a provider or an
/// effort the settings do not take is said on the settings page and counts as empty.
fn ai_settings(lookup: &mut Lookup) -> AiSettings {
    let mut choice = |key: &str, values: &'static [&'static str]| {
        lookup.first((key, Scope::User), |value| {
            let text = value
                .as_str()
                .ok_or(ProblemReason::WrongType { expected: "a string" })?;
            if text.is_empty() || values.contains(&text) {
                Ok(text.to_owned())
            } else {
                Err(ProblemReason::NotAChoice { choices: values })
            }
        })
    };
    let provider = choice(keys::AI_PROVIDER, &AiProvider::VALUES).and_then(|value| AiProvider::from_setting(&value));
    let effort = choice(keys::AI_EFFORT, &Effort::VALUES).and_then(|value| Effort::from_setting(&value));
    AiSettings {
        provider,
        endpoint: lookup.string(keys::AI_ENDPOINT, Scope::User).unwrap_or_default(),
        model: lookup.string(keys::AI_MODEL, Scope::User).unwrap_or_default(),
        effort,
    }
}

/// The settings of worktrees: `worktrees.folder`, `worktrees.openIn` and `terminal.external`. The home folder of the
/// user, for `~`, is set by the window when an operation starts, since reading the settings reads no environment.
fn worktree_settings(lookup: &mut Lookup) -> WorktreeSettings {
    WorktreeSettings {
        folder: lookup
            .string(keys::WORKTREES_FOLDER, Scope::UserAndRepository)
            .unwrap_or_default(),
        open_in: match lookup.choice(keys::WORKTREES_OPEN_IN, &keys::WORKTREES_OPEN_INS) {
            Some("editor") => WorktreeOpenIn::Editor,
            Some("terminal") => WorktreeOpenIn::Terminal,
            _ => WorktreeOpenIn::Window,
        },
        terminal: lookup.string(keys::EXTERNAL_TERMINAL, Scope::User).unwrap_or_default(),
        ..WorktreeSettings::default()
    }
}

/// `identities`: each entry with a name and an email that are text, in order; the others are left out.
fn identity_profiles(value: &Value) -> Result<Vec<model::contract::IdentityProfile>, ProblemReason> {
    let entries = value
        .as_array()
        .ok_or(ProblemReason::WrongType { expected: "a list" })?;
    Ok(entries
        .iter()
        .filter_map(|entry| {
            let text = |key: &str| entry.get(key).and_then(Value::as_str).map(str::to_owned);
            Some(model::contract::IdentityProfile {
                name: text("name")?,
                email: text("email")?,
                signing_key: text("signingKey"),
            })
        })
        .collect())
}
