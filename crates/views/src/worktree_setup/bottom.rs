//! The bottom of the page: where it saves, the box of VS Code's settings, and **Save**, **Discard changes**, **Bring
//! into…**, **Remove from…** and **Edit in settings file**.

use model::contract::{FeatureId, Target};
use model::worktree_setup::{SettingsPlace, SetupSave};
use ui::layout::{Align, Area, place};
use ui::{Button, ButtonKind, Choice, Rect, TextKind, Ui};

use super::rows::Row;
use super::{SetupIntent, SetupPageState};
use crate::feature::FeatureIntent;
use crate::intent::Intent;
use crate::text::worktree_setup as words;

/// An intent of the page.
fn page(intent: SetupIntent) -> Intent {
    Intent::WorktreeSetup(intent)
}

/// An operation run on what the page shows.
fn run(feature: &'static str, target: Target) -> Intent {
    Intent::Feature(FeatureIntent::Run(FeatureId(feature), target))
}

/// What the page would save, where it saves.
fn to_save(state: &SetupPageState<'_>) -> SetupSave {
    SetupSave {
        place: state.place,
        patterns: state.chosen.patterns.clone(),
        commands: state.commands.to_vec(),
        also_vs_code: state.also_vs_code && state.place == SettingsPlace::Repository,
    }
}

/// A row of the bottom of the page.
pub(super) fn bottom_row(state: &SetupPageState<'_>, row: Row, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    match row {
        Row::SaveIn => save_in(state, ui, rect),
        Row::AlsoVsCode => {
            let choice = Choice::check_box(words::ALSO_VS_CODE, state.also_vs_code);
            if ui.choice("also vs code", rect, &choice).clicked {
                return vec![page(SetupIntent::AlsoVsCode(!state.also_vs_code))];
            }
            Vec::new()
        }
        Row::Buttons => buttons(state, ui, rect),
        _ => Vec::new(),
    }
}

/// **Save in** and its two places.
fn save_in(state: &SetupPageState<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.large);
    let width = ui.measure(words::SAVE_IN, ui.text_style(TextKind::Strong));
    let _ = ui.label(
        parts.left(width),
        &ui::Label::new(words::SAVE_IN).kind(TextKind::Strong),
    );
    let mut intents = Vec::new();
    for (place_of, label) in [
        (SettingsPlace::Repository, words::REPOSITORY),
        (SettingsPlace::User, words::USER),
    ] {
        let mark = ui.theme().sizes.control_height;
        let width = ui.measure(label, ui.text_style(TextKind::Body)) + mark;
        let cell = parts.left(width);
        let choice = Choice::radio(label, state.place == place_of);
        if ui.choice(("save in", label), cell, &choice).clicked && state.place != place_of {
            intents.push(page(SetupIntent::ChoosePlace(place_of)));
        }
    }
    intents
}

/// The buttons at the bottom of the page.
fn buttons(state: &SetupPageState<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.medium);
    let mut intents = Vec::new();
    let add = |ui: &mut Ui<'_>, parts: &mut Area, (key, button): (&str, Button<'_>)| {
        let (width, height) = ui.button_size(&button);
        let cell = place(parts.left(width), (width, height), (Align::Start, Align::Center));
        ui.button(("bottom", key), cell, &button).clicked
    };
    let save = Button::primary(words::SAVE);
    let save = if state.saved.is_saved { save.disabled() } else { save };
    if add(ui, &mut parts, ("save", save)) {
        intents.push(page(SetupIntent::Save));
    }
    let discard = Button::secondary(words::DISCARD);
    let discard = if state.saved.has_changes {
        discard
    } else {
        discard.disabled()
    };
    if add(ui, &mut parts, ("discard", discard)) {
        intents.push(page(SetupIntent::Discard));
    }
    let bring_said = words::bring_into(state.other_worktrees);
    let has_patterns = !state.chosen.patterns.link.is_empty() || !state.chosen.patterns.copy.is_empty();
    let bring = Button::secondary(&bring_said);
    let bring = if state.saved.has_changes {
        bring.disabled().with_tooltip(words::SAVE_FIRST)
    } else if state.other_worktrees == 0 || !has_patterns {
        bring.disabled()
    } else {
        bring
    };
    if add(ui, &mut parts, ("bring", bring)) {
        intents.push(run("worktree-setup.bring-into-other-worktrees", Target::Head));
    }
    if state.saved.is_set_in_place {
        let remove = Button::secondary(words::remove_from(state.place))
            .kind(ButtonKind::Danger)
            .with_tooltip(words::REMOVE_FROM_TOOLTIP);
        if add(ui, &mut parts, ("remove", remove)) {
            intents.push(run(
                "worktree-setup.remove-from",
                Target::SetupSettings(Box::new(to_save(state))),
            ));
        }
    }
    let edit = Button::secondary(words::EDIT_IN_FILE).kind(ButtonKind::Outline);
    if add(ui, &mut parts, ("edit", edit)) {
        intents.push(run(
            "worktree-setup.edit-settings-file",
            Target::SetupSettings(Box::new(to_save(state))),
        ));
    }
    intents
}
