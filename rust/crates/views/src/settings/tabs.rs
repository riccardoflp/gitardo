//! The bar under the search of the settings page (`settings.edit`): the tabs **User** and **Repository**, which choose
//! the file a change is written in, and **Edit in settings.json**, which opens that file in a File page.

use model::contract::{FeatureId, Target};
use model::worktree_setup::{SettingsPlace, SetupSave};
use ui::layout::{Align, Area, place};
use ui::{AccessRole, Button, ButtonKind, Description, Label, Rect, Sense, TextKind, Tone, Ui};

use super::{SettingsPageState, SettingsTarget};
use crate::feature::FeatureIntent;
use crate::intent::Intent;
use crate::text::settings as words;

/// The tabs of the files, the one shown underlined in the color of the accent, **Repository** only while a
/// repository is shown, and **Edit in settings.json** on the right; a line runs under the whole bar.
pub(super) fn target_bar(state: &SettingsPageState<'_>, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<Intent>) {
    let theme = *ui.theme();
    let border = theme.sizes.border;
    ui.fill_rect(
        Rect::new(rect.x, rect.bottom() - border, rect.width, border),
        theme.colors.border,
    );
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.small);
    let edit = Button::secondary(words::EDIT_IN_FILE).kind(ButtonKind::Outline);
    let (width, height) = ui.button_size(&edit);
    let cell = place(parts.right(width), (width, height), (Align::Start, Align::Start));
    if ui.button("edit settings file", cell, &edit).clicked {
        intents.push(edit_file(state.target));
    }
    let mut tabs = vec![(SettingsTarget::User, words::USER_TAB)];
    if state.has_repository {
        tabs.push((SettingsTarget::Repository, words::REPOSITORY_TAB));
    }
    for (target, label) in tabs {
        let width = ui.measure(label, ui.text_style(TextKind::Medium)) + theme.sizes.spacing.large * 2.0;
        if file_tab(ui, parts.left(width), (label, state.target == target)) && state.target != target {
            intents.push(Intent::ChooseSettingsFile(target));
        }
    }
}

/// A tab of a file: its name, strong while it is shown or under the pointer, over a line in the color of the accent
/// while it is shown; says whether it was clicked.
fn file_tab(ui: &mut Ui<'_>, rect: Rect, (label, is_shown): (&str, bool)) -> bool {
    let theme = *ui.theme();
    let id = ui.id(("settings file tab", label));
    let response = ui.interact(id, rect, Sense::CLICK);
    let tone = if is_shown || response.hovered {
        Tone::Normal
    } else {
        Tone::Muted
    };
    let text = Label::new(label).kind(TextKind::Medium).tone(tone).align(Align::Center);
    let _ = ui.label(rect, &text);
    if is_shown {
        let thickness = theme.sizes.border * 2.0;
        let line = Rect::new(rect.x, rect.bottom() - thickness, rect.width, thickness);
        ui.fill_rect(line, theme.colors.accent);
    }
    ui.describe(id, rect, &Description::new(AccessRole::Tab, label).selected(is_shown));
    response.clicked
}

/// The intent of **Edit in settings.json**: the operation that opens the file chosen, made first when the repository
/// has none.
fn edit_file(target: SettingsTarget) -> Intent {
    let place = match target {
        SettingsTarget::User => SettingsPlace::User,
        SettingsTarget::Repository => SettingsPlace::Repository,
    };
    let save = SetupSave {
        place,
        ..SetupSave::default()
    };
    let feature = FeatureId("worktree-setup.edit-settings-file");
    Intent::Feature(FeatureIntent::Run(feature, Target::SetupSettings(Box::new(save))))
}
