//! The entries of the upstream of a local branch in its menu (`branch.set-upstream`, `branch.unset-upstream`).

use model::contract::OpenIn;

use super::{of_branch, repository};

#[test]
fn branch_set_upstream__a_branch_with_an_upstream_has_change_upstream_and_one_with_none_set_upstream() {
    let mut repository = repository();
    assert!(of_branch(&repository, OpenIn::Checkout, "feature").contains(&String::from("Change upstream\u{2026}")));
    if let Some(feature) = repository.branches.iter_mut().find(|branch| branch.name == "feature") {
        feature.upstream = None;
    }
    let entries = of_branch(&repository, OpenIn::Checkout, "feature");
    assert!(entries.contains(&String::from("Set upstream\u{2026}")), "{entries:?}");
    assert!(
        !entries.iter().any(|entry| entry.starts_with("Stop tracking")),
        "{entries:?}"
    );
}

#[test]
fn branch_set_upstream__a_repository_with_no_remote_branch_has_neither_entry() {
    let mut repository = repository();
    repository.remote_branches.clear();
    if let Some(feature) = repository.branches.iter_mut().find(|branch| branch.name == "feature") {
        feature.upstream = None;
    }
    let entries = of_branch(&repository, OpenIn::Checkout, "feature");
    assert!(!entries.iter().any(|entry| entry.contains("upstream")), "{entries:?}");
}

#[test]
fn branch_unset_upstream__a_branch_whose_upstream_is_gone_has_stop_tracking() {
    let mut repository = repository();
    if let Some(upstream) = repository
        .branches
        .iter_mut()
        .find(|branch| branch.name == "feature")
        .and_then(|branch| branch.upstream.as_mut())
    {
        upstream.is_gone = true;
    }
    let entries = of_branch(&repository, OpenIn::Checkout, "feature");
    assert!(
        entries.contains(&String::from("Stop tracking origin/feature")),
        "{entries:?}"
    );
    assert!(
        entries.contains(&String::from("Change upstream\u{2026}")),
        "{entries:?}"
    );
}
