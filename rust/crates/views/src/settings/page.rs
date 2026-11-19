//! What the window keeps of the settings page between frames, and what is asked of it: what is typed in its search
//! (`settings.search`), whether it lists the sections, the settings changed or the experimental ones
//! (`settings.sections`), and where its list was last asked to go (`settings.reveal`).

use super::search::find;
use super::{SettingRow, SettingSource};
use crate::text::settings::SettingsSection;

/// A place of the list of the settings page: the name of a section, or the row of a setting by its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingsAnchor {
    /// The name of a section.
    Section(SettingsSection),
    /// The row of a setting.
    Setting(&'static str),
}

/// What the list of the settings page holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SettingsList {
    /// Every section, each under its name.
    #[default]
    Sections,
    /// **Changed**: only the settings a file sets, in the order of the card.
    Changed,
    /// **Experimental**: only the experimental settings, in the order of the card.
    Experimental,
}

/// Something asked of the settings page itself.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SettingsIntent {
    /// Scroll the list to the name of a section, or to the row of a setting, listing the sections (`settings.reveal`).
    Reveal(SettingsAnchor),
    /// What is typed in the search changed; empty when `Escape` emptied it (`settings.search`).
    Search(String),
    /// **Changed** or **Experimental** clicked: lists only those settings, or the sections again when that list is
    /// shown already (`settings.sections`).
    Show(SettingsList),
}

/// What the window keeps of the settings page.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SettingsPage {
    /// What is typed in the search.
    pub query: String,
    /// What the list holds.
    pub list: SettingsList,
    /// The place the list was last asked to go to, and the number of that request, so that the list goes there once;
    /// none once the list changed since.
    pub reveal: Option<(SettingsAnchor, u64)>,
    /// The number of the last change of what is typed, after which the list of what is found starts at its top.
    pub searched: u64,
    /// How many times the page was shown, so that its search takes the keyboard each time.
    pub opened: u64,
    /// How many requests were made.
    pub serial: u64,
}

impl SettingsPage {
    /// The page after something asked of it.
    pub fn apply(&mut self, intent: SettingsIntent) {
        self.serial += 1;
        match intent {
            SettingsIntent::Reveal(anchor) => {
                self.list = SettingsList::Sections;
                self.reveal = Some((anchor, self.serial));
            }
            SettingsIntent::Search(query) => {
                self.query = query;
                self.searched = self.serial;
                self.reveal = None;
            }
            SettingsIntent::Show(list) => {
                self.list = if self.list == list {
                    SettingsList::Sections
                } else {
                    list
                };
                self.reveal = None;
            }
        }
    }

    /// The page shown again: its search takes the keyboard.
    pub fn shown(&mut self) {
        self.opened += 1;
    }

    /// The rows the list shows, by their place among the rows of the page: those of what it holds that answer the
    /// search, in the order of the sections then of the card, or of the card alone for **Changed** and
    /// **Experimental**.
    #[must_use]
    pub fn listed(&self, rows: &[SettingRow]) -> Vec<usize> {
        let found = find(rows, &self.query).rows;
        let is_copilot = rows
            .iter()
            .any(|row| row.info.key == "ai.provider" && row.text.trim() == "copilot");
        let is_kept = |index: &usize| {
            rows.get(*index)
                .is_some_and(|row| !(is_copilot && row.info.key == "ai.endpoint"))
                && rows.get(*index).is_some_and(|row| match self.list {
                    SettingsList::Sections => true,
                    SettingsList::Changed => is_set(row),
                    SettingsList::Experimental => row.info.is_experimental,
                })
        };
        let mut listed: Vec<usize> = found.into_iter().filter(is_kept).collect();
        if self.list == SettingsList::Sections {
            listed.sort_by_key(|index| rows.get(*index).and_then(|row| row.info.section));
        }
        listed
    }
}

/// Whether a file sets a setting, taken or not.
#[must_use]
pub fn is_set(row: &SettingRow) -> bool {
    row.source != SettingSource::Default || row.problem.is_some()
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::*;
    use crate::settings::default_rows;

    /// Whether the page lists a setting, with `ai.provider` set to a value.
    fn lists(provider: &str, key: &str) -> bool {
        let mut rows = default_rows();
        for row in &mut rows {
            if row.info.key == "ai.provider" {
                row.text = provider.to_owned();
            }
        }
        let page = SettingsPage::default();
        page.listed(&rows)
            .iter()
            .any(|index| rows.get(*index).is_some_and(|row| row.info.key == key))
    }

    #[test]
    fn ai_copilot__the_settings_page_leaves_out_the_endpoint_while_the_provider_is_copilot() {
        assert!(lists("anthropic", "ai.endpoint"));
        assert!(!lists("copilot", "ai.endpoint"));
        assert!(lists("copilot", "ai.model"));
    }
}
