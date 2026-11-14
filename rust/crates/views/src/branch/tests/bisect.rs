//! The entries of the menu of a commit while a bisect is under way.

use super::{of_commit, repository};

#[test]
fn message_edit__while_a_bisect_is_under_way_edit_message_is_not_offered() {
    let mut repository = repository();
    repository.bisect = Some(model::bisect::Bisect::default());
    assert!(!of_commit(&repository, 'a').contains(&String::from("Edit message\u{2026}")));
}
