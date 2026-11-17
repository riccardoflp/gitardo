//! A Diff page beside the inspector (`diff.beside-inspector`): a file of the commit in the inspector opens its diff
//! where the timeline was, the inspector staying on the right, and `Alt+Down` and `Alt+Up` go to the next and the
//! previous file of the inspector in the same page.

use ui::{Key, Modifiers, NamedKey, Ui};

use super::DiffIntent;
use crate::intent::Intent;
use crate::page::{PageHostState, host_view};

/// The Diff page shown beside the inspector, with its trail.
#[derive(Clone, Debug, PartialEq)]
pub struct BesideInspectorState<'a> {
    /// The page, with its trail.
    pub host: PageHostState<'a>,
}

/// The way `Alt+Down` and `Alt+Up` go among the files of the inspector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FileStep {
    /// The next file, under the one shown.
    Next,
    /// The previous file, above the one shown.
    Previous,
}

/// The Diff page opened from the inspector, over the rectangle its caller clipped it to, where the timeline was: its
/// trail and the page, and the keys that go from file to file of the inspector.
///
/// Feature: diff.beside-inspector
pub fn beside_inspector_view(state: &BesideInspectorState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = host_view(&state.host, ui);
    for key in ui.unclaimed_keys() {
        let step = match (key.key, key.modifiers) {
            (Key::Named(NamedKey::ArrowDown), Modifiers::ALT) => FileStep::Next,
            (Key::Named(NamedKey::ArrowUp), Modifiers::ALT) => FileStep::Previous,
            _ => continue,
        };
        intents.push(Intent::Diff(DiffIntent::StepBeside(step)));
    }
    intents
}
