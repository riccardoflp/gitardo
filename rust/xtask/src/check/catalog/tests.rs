use super::{Catalog, problems};
use crate::catalog::{Card, FeatureFunction, IndexRow};

fn card(identifier: &str, function: &str, acceptance: usize) -> Card {
    Card {
        identifier: identifier.to_owned(),
        kind: String::from("Operation"),
        function: function.to_owned(),
        acceptance,
        file: String::from("docs/features/06.md"),
        line: 10,
    }
}

fn row(identifier: &str, status: &str) -> IndexRow {
    IndexRow {
        identifier: identifier.to_owned(),
        kind: String::from("Operation"),
        status: status.to_owned(),
        extension: None,
        line: 3,
    }
}

fn function(identifier: &str, file: &str, name: &str) -> FeatureFunction {
    FeatureFunction {
        identifier: identifier.to_owned(),
        name: Some(name.to_owned()),
        file: file.to_owned(),
        line: 5,
    }
}

fn test(name: &str) -> (String, usize, String) {
    (
        String::from("crates/features/src/branch/create.rs"),
        40,
        name.to_owned(),
    )
}

fn built_catalog() -> Catalog {
    Catalog {
        cards: vec![card("branch.create", "features::branch::create", 2)],
        index: vec![row("branch.create", "Built")],
        functions: vec![function(
            "branch.create",
            "crates/features/src/branch/create.rs",
            "create",
        )],
        tests: vec![
            test("branch_create__makes_the_branch"),
            test("branch_create__refuses_a_bad_name"),
        ],
    }
}

fn messages(catalog: &Catalog) -> Vec<String> {
    problems(catalog)
        .into_iter()
        .map(|problem| format!("{} {}", problem.rule, problem.message))
        .collect()
}

#[test]
fn passes_a_card_that_is_built_with_its_function_and_tests() {
    assert_eq!(messages(&built_catalog()), Vec::<String>::new());
}

#[test]
fn passes_a_card_only_specified_without_code() {
    let catalog = Catalog {
        cards: vec![card("branch.create", "features::branch::create", 2)],
        index: vec![row("branch.create", "Specified")],
        functions: Vec::new(),
        tests: Vec::new(),
    };
    assert_eq!(messages(&catalog), Vec::<String>::new());
}

#[test]
fn refuses_a_built_card_with_fewer_tests_than_acceptance_lines() {
    let mut catalog = built_catalog();
    let _ = catalog.tests.pop();
    let found = messages(&catalog);
    assert_eq!(found.len(), 1);
    assert!(
        found[0].starts_with("R2.6 branch.create is Built with 1 tests"),
        "{found:?}"
    );
}

#[test]
fn refuses_a_built_card_without_its_function() {
    let mut catalog = built_catalog();
    catalog.functions.clear();
    assert_eq!(
        messages(&catalog),
        vec![String::from(
            "R2.1 branch.create is Built but no function says `/// Feature: branch.create`."
        )]
    );
}

#[test]
fn refuses_a_function_in_another_crate_or_with_another_name() {
    let mut catalog = built_catalog();
    catalog.functions = vec![function("branch.create", "crates/model/src/branch.rs", "create")];
    assert!(messages(&catalog)[0].contains("belongs in the crate features"));
    catalog.functions = vec![function(
        "branch.create",
        "crates/features/src/branch/create.rs",
        "make",
    )];
    assert!(messages(&catalog)[0].contains("is create in its card, not make"));
}

#[test]
fn refuses_a_function_or_a_test_of_no_card() {
    let mut catalog = built_catalog();
    catalog
        .functions
        .push(function("branch.forge", "crates/features/src/branch/forge.rs", "forge"));
    catalog.tests.push(test("branch_forge__works"));
    let found = messages(&catalog);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found
            .iter()
            .any(|message| message.contains("branch.forge is not a card"))
    );
    assert!(
        found
            .iter()
            .any(|message| message.contains("branch_forge, which is no card"))
    );
}

#[test]
fn refuses_an_index_that_does_not_match_the_cards() {
    let mut catalog = built_catalog();
    catalog.index.push(row("branch.gone", "Specified"));
    catalog.index[0].kind = String::from("View");
    catalog.cards.push(card("branch.create", "features::branch::create", 2));
    let found = messages(&catalog);
    assert!(
        found.iter().any(|message| message.contains("a second card")),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|message| message.contains("is of the kind Operation in its card and View in the index"))
    );
    assert!(
        found
            .iter()
            .any(|message| message.contains("The index names branch.gone"))
    );
}

#[test]
fn refuses_an_unknown_status() {
    let mut catalog = built_catalog();
    catalog.index[0].status = String::from("Done");
    assert!(
        messages(&catalog)
            .iter()
            .any(|message| message.contains("has the status Done"))
    );
}

#[test]
fn passes_the_statuses_of_the_extension_behind_the_desktop() {
    let mut catalog = built_catalog();
    for cell in ["Planned", "Built", "Seen", "Differs (A5, D7)", "Host only"] {
        catalog.index[0].extension = Some(cell.to_owned());
        assert!(messages(&catalog).is_empty(), "{cell}: {:?}", messages(&catalog));
    }
}

#[test]
fn refuses_an_unknown_status_in_the_extension() {
    let mut catalog = built_catalog();
    catalog.index[0].extension = Some(String::from("Done"));
    assert!(
        messages(&catalog)
            .iter()
            .any(|message| message.contains("has the status Done in the extension"))
    );
}

#[test]
fn refuses_a_card_built_in_the_extension_before_the_desktop() {
    let mut catalog = built_catalog();
    catalog.index[0].status = String::from("Specified");
    catalog.index[0].extension = Some(String::from("Differs (B3)"));
    assert!(
        messages(&catalog)
            .iter()
            .any(|message| message.contains("is Differs in the extension and Specified in Gitardo Desktop"))
    );
    catalog.index[0].extension = Some(String::from("Planned"));
    assert!(
        !messages(&catalog)
            .iter()
            .any(|message| message.contains("the desktop comes first"))
    );
}
