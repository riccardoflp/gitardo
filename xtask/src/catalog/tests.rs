use super::{feature_test_names, parse_cards, parse_feature_functions, parse_index, test_prefix};

const AREA: &str = "# Branches

Intro with a list:

- [ ] not a card line

### `branch.create` — Create a branch

- **Kind**: Operation
- **Function**: `features::branch::create(context, answers) -> Outcome`

Acceptance:

- [ ] The branch is made.
- [ ] A name git refuses is refused
      while it is typed.

Rules after the list:

- [ ] not an acceptance line

### `branch.open` — What a double click does

- **Kind**: Derivation (a note)
- **Function**: `model::branch::open(input: &OpenBranchInput) -> Option<Intent>`

## Another section

- [ ] not either
";

#[test]
fn reads_the_cards_of_an_area() {
    let cards = parse_cards(AREA, "docs/features/06.md");
    assert_eq!(cards.len(), 2);
    assert_eq!(cards[0].identifier, "branch.create");
    assert_eq!(cards[0].kind, "Operation");
    assert_eq!(cards[0].function, "features::branch::create");
    assert_eq!(cards[0].acceptance, 2);
    assert_eq!(cards[0].line, 7);
    assert_eq!(cards[1].kind, "Derivation");
    assert_eq!(cards[1].function, "model::branch::open");
    assert_eq!(cards[1].acceptance, 0);
}

#[test]
fn reads_the_rows_of_the_index() {
    let index = "| File | Area |\n| --- | --- |\n\
        | [`branch.create`](06.md#x) | Create a branch | Operation | Built |\n\
        | [`branch.open`](06.md#y) | What a double click does | Derivation | Specified |\n";
    let rows = parse_index(index);
    assert_eq!(rows.len(), 2);
    assert_eq!(
        (
            rows[0].identifier.as_str(),
            rows[0].kind.as_str(),
            rows[0].status.as_str()
        ),
        ("branch.create", "Operation", "Built")
    );
    assert_eq!(rows[1].line, 4);
}

#[test]
fn finds_the_functions_that_name_their_feature() {
    let source = "/// Creates a branch.\n///\n/// Feature: branch.create\n#[must_use]\npub fn create(context: &Context) {}\n\
        /// Feature: branch.open\nconst X: u8 = 1;\n";
    let found = parse_feature_functions(source, "crates/features/src/branch/create.rs");
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].identifier, "branch.create");
    assert_eq!(found[0].name.as_deref(), Some("create"));
    assert_eq!(found[0].line, 3);
    assert_eq!(found[1].name, None);
}

#[test]
fn names_tests_after_the_identifier() {
    assert_eq!(test_prefix("graph.fold-merge"), "graph_fold_merge");
    let names = feature_test_names(
        "#[test]\nfn branch_create__refuses_a_name() {}\nfn plain() {}\nfn graph_view__draws() {}\n",
    );
    assert_eq!(
        names,
        vec![
            (2, String::from("branch_create__refuses_a_name")),
            (4, String::from("graph_view__draws"))
        ]
    );
}
