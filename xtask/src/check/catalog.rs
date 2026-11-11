//! Rules R2.1 and R2.6: the catalog and the code name each other. Every function that says it is a feature has a
//! card, of the crate and name the card gives; every card the index marks built has its function, and at least as many
//! tests named after it as its acceptance list has lines; the index and the cards agree.

use std::collections::BTreeMap;

use super::Workspace;
use crate::catalog::{
    Card, FeatureFunction, IndexRow, feature_test_names, parse_cards, parse_feature_functions, parse_index, test_prefix,
};
use crate::files::{has_extension, is_rust};
use crate::problem::Problem;
use crate::source::code_only;

/// The index of the catalog.
const INDEX: &str = "docs/features/README.md";

/// The statuses a card can have, in the order it earns them.
const STATUSES: [&str; 4] = ["Specified", "Reviewed", "Built", "Seen"];

/// The statuses that say the code of a card is written.
const BUILT: [&str; 2] = ["Built", "Seen"];

/// What the check reads, gathered from the documents and the code.
pub struct Catalog {
    /// Every card of every area.
    pub cards: Vec<Card>,
    /// The rows of the index.
    pub index: Vec<IndexRow>,
    /// The functions that say which feature they are.
    pub functions: Vec<FeatureFunction>,
    /// The names of the tests of features, with their file and line.
    pub tests: Vec<(String, usize, String)>,
}

/// Reads the catalog and the code of the workspace.
pub fn read(workspace: &Workspace) -> Catalog {
    let mut catalog = Catalog {
        cards: Vec::new(),
        index: Vec::new(),
        functions: Vec::new(),
        tests: Vec::new(),
    };
    for (file, text) in &workspace.files {
        let (relative, Some(text)) = (file.relative.as_str(), text) else {
            continue;
        };
        if relative == INDEX {
            catalog.index = parse_index(text);
        } else if is_area(relative) {
            catalog.cards.extend(parse_cards(text, relative));
        } else if relative.starts_with("crates/") && is_rust(file) {
            catalog.functions.extend(parse_feature_functions(text, relative));
            let names = feature_test_names(&code_only(text));
            catalog
                .tests
                .extend(names.into_iter().map(|(line, name)| (relative.to_owned(), line, name)));
        }
    }
    catalog
}

/// Whether a file is one of the areas of the catalog: `docs/features/06-branches-tags-remotes.md`.
fn is_area(relative: &str) -> bool {
    relative
        .strip_prefix("docs/features/")
        .is_some_and(|name| has_extension(name, "md") && name.starts_with(|first: char| first.is_ascii_digit()))
}

/// The problems of the catalog of the workspace.
pub fn check(workspace: &Workspace) -> Vec<Problem> {
    problems(&read(workspace))
}

/// The problems of a catalog.
pub fn problems(catalog: &Catalog) -> Vec<Problem> {
    let mut found = card_problems(catalog);
    found.extend(function_problems(catalog));
    found.extend(built_problems(catalog));
    found.extend(test_problems(catalog));
    found
}

fn card_problems(catalog: &Catalog) -> Vec<Problem> {
    let mut found = Vec::new();
    let mut seen: BTreeMap<&str, &Card> = BTreeMap::new();
    for card in &catalog.cards {
        if let Some(first) = seen.insert(&card.identifier, card) {
            let message = format!(
                "{} has a second card; the first is at {}:{}.",
                card.identifier, first.file, first.line
            );
            found.push(Problem::at_line("R2.1", &card.file, card.line, message));
        }
        let row = catalog.index.iter().find(|row| row.identifier == card.identifier);
        let message = match row {
            None => format!("{} has no row in the index of the catalog.", card.identifier),
            Some(row) if row.kind != card.kind => {
                format!(
                    "{} is of the kind {} in its card and {} in the index.",
                    card.identifier, card.kind, row.kind
                )
            }
            Some(_) => continue,
        };
        found.push(Problem::at_line("R2.1", &card.file, card.line, message));
    }
    for row in &catalog.index {
        let message = if !seen.contains_key(row.identifier.as_str()) {
            format!("The index names {}, which has no card.", row.identifier)
        } else if !STATUSES.contains(&row.status.as_str()) {
            format!(
                "{} has the status {}, which is not one of {}.",
                row.identifier,
                row.status,
                STATUSES.join(", ")
            )
        } else {
            continue;
        };
        found.push(Problem::at_line("R2.1", INDEX, row.line, message));
    }
    found
}

fn function_problems(catalog: &Catalog) -> Vec<Problem> {
    let mut found = Vec::new();
    let mut seen: BTreeMap<&str, &FeatureFunction> = BTreeMap::new();
    for function in &catalog.functions {
        let card = catalog.cards.iter().find(|card| card.identifier == function.identifier);
        let message = match (card, seen.insert(&function.identifier, function)) {
            (None, _) => format!("{} is not a card of the catalog.", function.identifier),
            (Some(_), Some(first)) => {
                format!(
                    "{} has a second function; the first is at {}:{}.",
                    function.identifier, first.file, first.line
                )
            }
            (Some(card), None) => match mismatch(card, function) {
                Some(message) => message,
                None => continue,
            },
        };
        found.push(Problem::at_line("R2.1", &function.file, function.line, message));
    }
    found
}

/// What differs between the function a card names and the function that says it is that card.
fn mismatch(card: &Card, function: &FeatureFunction) -> Option<String> {
    let mut path = card.function.split("::");
    let crate_name = path.next().unwrap_or_default();
    let name = card.function.rsplit("::").next().unwrap_or_default();
    let in_crate = function
        .file
        .strip_prefix("crates/")
        .is_some_and(|rest| rest.split('/').next() == Some(crate_name));
    if !in_crate {
        return Some(format!(
            "{} belongs in the crate {crate_name}, as its card says.",
            function.identifier
        ));
    }
    match &function.name {
        Some(found) if found == name => None,
        Some(found) => Some(format!(
            "The function of {} is {name} in its card, not {found}.",
            function.identifier
        )),
        None => Some(format!(
            "`/// Feature: {}` is not followed by a function.",
            function.identifier
        )),
    }
}

fn built_problems(catalog: &Catalog) -> Vec<Problem> {
    let mut found = Vec::new();
    for row in catalog.index.iter().filter(|row| BUILT.contains(&row.status.as_str())) {
        let Some(card) = catalog.cards.iter().find(|card| card.identifier == row.identifier) else {
            continue;
        };
        if !catalog
            .functions
            .iter()
            .any(|function| function.identifier == card.identifier)
        {
            let message = format!(
                "{} is {} but no function says `/// Feature: {}`.",
                card.identifier, row.status, card.identifier
            );
            found.push(Problem::at_line("R2.1", &card.file, card.line, message));
        }
        let prefix = format!("{}__", test_prefix(&card.identifier));
        let tests = catalog
            .tests
            .iter()
            .filter(|(_, _, name)| name.starts_with(&prefix))
            .count();
        let wanted = card.acceptance.max(1);
        if tests < wanted {
            let message = format!(
                "{} is {} with {tests} tests named {prefix}…, and its acceptance list asks for at least {wanted}.",
                card.identifier, row.status
            );
            found.push(Problem::at_line("R2.6", &card.file, card.line, message));
        }
    }
    found
}

fn test_problems(catalog: &Catalog) -> Vec<Problem> {
    let prefixes: Vec<String> = catalog.cards.iter().map(|card| test_prefix(&card.identifier)).collect();
    let mut found = Vec::new();
    for (file, line, name) in &catalog.tests {
        let prefix = name.split("__").next().unwrap_or_default();
        if !prefixes.iter().any(|known| known == prefix) {
            let message = format!("The test {name} is named after {prefix}, which is no card of the catalog.");
            found.push(Problem::at_line("R2.6", file, *line, message));
        }
    }
    found
}

#[cfg(test)]
mod tests;
