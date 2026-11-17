//! The picker of the themes (`settings.theme`): the five of `theme`, the one in force said as current.

use model::contract::{Answer, AnswerKey, Answers, FeatureId, Target};
use ui::{Picker, PickerAnswer, PickerItem, PickerMode, Ui};

use crate::feature::FeatureIntent;
use crate::intent::Intent;
use crate::text::settings as words;

/// What the picker of the themes shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemePickerState<'a> {
    /// The value of `theme` in force: `system`.
    pub current: &'a str,
}

/// The picker of the themes: the five of `theme`, the one in force said as current; the one picked is written in the
/// user's settings file by `settings.theme`, and the window is drawn with it as soon as the file is read again.
pub fn theme_picker_view(state: &ThemePickerState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let descriptions: Vec<&str> = words::THEME_VALUES
        .iter()
        .map(|value| {
            if *value == state.current {
                words::CURRENT_THEME
            } else {
                ""
            }
        })
        .collect();
    let items: Vec<PickerItem<'_>> = words::THEME_NAMES
        .iter()
        .zip(&descriptions)
        .map(|(label, description)| PickerItem { label, description })
        .collect();
    let picker = Picker {
        placeholder: words::CHOOSE_THEME,
        items: &items,
        mode: PickerMode::Single,

        query: "",
        checked: &[],
    };
    match ui.picker("theme picker", &picker) {
        Some(PickerAnswer::Chose(index)) => match words::THEME_VALUES.get(index) {
            Some(value) => {
                let answers = Answers::new().with(AnswerKey("theme"), Answer::Choice((*value).to_owned()));
                let run = FeatureIntent::RunWith(FeatureId("settings.theme"), Target::Head, answers);
                vec![Intent::Feature(run), Intent::CloseThemePicker]
            }
            None => vec![Intent::CloseThemePicker],
        },
        Some(PickerAnswer::Cancelled | PickerAnswer::ChoseMany(_)) => vec![Intent::CloseThemePicker],
        None => Vec::new(),
    }
}
