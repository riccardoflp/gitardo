//! The search of the settings page (`settings.search`): the settings that answer what is typed, each word found,
//! whatever its case, in the title, the key, the description or the value of a setting.

use super::SettingRow;
use crate::text::settings::SettingsSection;

/// The settings that answer a search.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Found {
    /// The rows that answer, by their place among the rows of the page, in its order.
    pub rows: Vec<usize>,
}

impl Found {
    /// How many of the rows that answer are in a section.
    #[must_use]
    pub fn count_in(&self, section: SettingsSection, rows: &[SettingRow]) -> usize {
        self.rows
            .iter()
            .filter(|index| rows.get(**index).is_some_and(|row| row.info.section == Some(section)))
            .count()
    }
}

/// The settings that answer what is typed: those in a section in which every word typed is found, whatever its case,
/// in the title, the key, the description or the value; every one in a section when nothing is typed.
///
/// Feature: settings.search
#[must_use]
pub fn find(rows: &[SettingRow], query: &str) -> Found {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let rows = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.info.section.is_some())
        .filter(|(_, row)| {
            let said = [row.info.title, row.info.key, row.info.description, row.value.as_str()].map(str::to_lowercase);
            words
                .iter()
                .all(|word| said.iter().any(|text| text.contains(word.as_str())))
        })
        .map(|(index, _)| index)
        .collect();
    Found { rows }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::*;
    use crate::settings::default_rows;

    /// The keys of the settings that answer a search of every setting at its default.
    fn keys(query: &str) -> Vec<&'static str> {
        let rows = default_rows();
        find(&rows, query)
            .rows
            .iter()
            .filter_map(|index| rows.get(*index).map(|row| row.info.key))
            .collect()
    }

    #[test]
    fn settings_search__page_size_finds_graph_page_size_by_its_key() {
        assert_eq!(keys("page size"), vec!["graph.pageSize"]);
        assert_eq!(
            keys("PAGE   Size"),
            vec!["graph.pageSize"],
            "whatever the case and the spaces"
        );
    }

    #[test]
    fn settings_search__rebase_finds_pull_rebase_by_its_title_and_nothing_in_branches() {
        let rows = default_rows();
        let found = find(&rows, "rebase");
        assert!(keys("rebase").contains(&"pull.rebase"));
        assert_eq!(found.count_in(SettingsSection::Branches, &rows), 0);
        let title = rows
            .iter()
            .find(|row| row.info.key == "pull.rebase")
            .map(|row| row.info.title);
        assert_eq!(title, Some("Pull with rebase"));
    }

    #[test]
    fn settings_search__a_color_finds_the_old_color_of_the_blame_by_its_value() {
        assert_eq!(keys("#316dca"), vec!["fileBlame.age.oldColor"]);
    }
}
