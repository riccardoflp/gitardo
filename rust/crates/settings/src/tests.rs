//! The lookup of the two files and the settings of the operations, as the engine reads them without the window. The
//! tests of the card `settings.read` are in `app`, whose function it names.

use model::contract::CommandPreview;
use model::graph::PageSize;
use model::refs::SortOrder;

use crate::keys::{self, Scope};
use crate::{Lookup, ProblemReason, SettingsFile, feature_settings, page_size, sort_order};

#[test]
fn the_repositorys_file_comes_first_for_a_setting_it_may_set() {
    let mut lookup = Lookup::parse(
        Some(r#"{ "graph.pageSize": 300, "pull.rebase": false }"#),
        Some(r#"{ "graph.pageSize": 200, "pull.rebase": true }"#),
    );
    assert_eq!(page_size(&mut lookup), PageSize::from_setting(Some(200.0)));
    assert!(feature_settings(&mut lookup).pull_rebase);
    assert_eq!(lookup.into_problems(), []);
}

#[test]
fn a_setting_of_the_user_alone_set_by_the_repository_is_not_taken_and_is_noted() {
    let mut lookup = Lookup::parse(Some("{}"), Some(r#"{ "commandPreview": "all" }"#));
    assert_eq!(feature_settings(&mut lookup).command_preview, CommandPreview::Off);
    let problems = lookup.into_problems();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(
        problems.first().map(|problem| problem.key.as_str()),
        Some(keys::COMMAND_PREVIEW)
    );
    assert_eq!(
        problems.first().map(|problem| problem.file),
        Some(SettingsFile::Repository)
    );
}

#[test]
fn a_value_of_the_wrong_kind_leaves_the_default_and_says_what_was_expected() {
    let mut lookup = Lookup::parse(Some(r#"{ "branches.sortOrder": 3, "pull.autostash": "yes" }"#), None);
    assert_eq!(sort_order(&mut lookup), None);
    assert_eq!(lookup.boolean(keys::PULL_AUTOSTASH, Scope::UserAndRepository), None);
    let reasons: Vec<ProblemReason> = lookup
        .into_problems()
        .into_iter()
        .map(|problem| problem.reason)
        .collect();
    assert_eq!(
        reasons,
        [
            ProblemReason::WrongType { expected: "a string" },
            ProblemReason::WrongType {
                expected: "true or false"
            }
        ]
    );
}

#[test]
fn the_order_of_the_branches_is_by_date_only_when_it_says_so() {
    let order = |text: &str| sort_order(&mut Lookup::parse(Some(text), None));
    assert_eq!(order(r#"{ "branches.sortOrder": "date" }"#), Some(SortOrder::Date));
    assert_eq!(order(r#"{ "branches.sortOrder": "name" }"#), Some(SortOrder::Name));
    assert_eq!(order("{}"), None);
}

#[test]
fn a_text_that_is_not_json_sets_nothing_and_is_noted() {
    let lookup = Lookup::parse(Some("{ not json"), None);
    assert!(lookup.written_values().is_empty());
    let problems = lookup.into_problems();
    assert!(matches!(
        problems.as_slice(),
        [problem] if problem.key.is_empty() && matches!(problem.reason, ProblemReason::Unreadable(_))
    ));
}

#[test]
fn rebase_update_refs_is_read_from_either_file_and_is_none_when_not_set() {
    let mut set = Lookup::parse(None, Some(r#"{ "rebase.updateRefs": false }"#));
    assert_eq!(feature_settings(&mut set).rebase_update_refs, Some(false));
    let mut not_set = Lookup::parse(Some("{}"), None);
    assert_eq!(feature_settings(&mut not_set).rebase_update_refs, None);
}
