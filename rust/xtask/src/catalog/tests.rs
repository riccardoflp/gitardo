use super::{
    feature_test_names, names_test, parse_cards, parse_feature_functions, parse_index, parse_tested_by, test_prefix,
};

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
- [ ] Extension: the extension's own line, which its tests check.

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
fn finds_a_function_after_an_attribute_on_several_lines() {
    let source = "/// Feature: branch.validate-name\n#[allow(\n    clippy::x,\n    reason = \"[a]\"\n)]\npub fn validate_name() {}\n";
    let found = parse_feature_functions(source, "crates/model/src/branch.rs");
    assert_eq!(found[0].name.as_deref(), Some("validate_name"));
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

#[test]
fn reads_the_files_and_names_after_tested_by_to_the_end_of_the_sentence() {
    let text = "- **Extension**: kept in memory. Tested by `test/e2e/ui.mjs`, step \"with keeping on, it\n  shows the commit\"; `test/kept.test.ts`; `rust/crates/server/tests/stdio/summaries.rs`, *reads once*,\n  and **Cancel** names nothing. Then `test/other.test.ts` is not tested by.\n- **Uses**: `test/uses.test.ts`.\n";
    let found = parse_tested_by(text, "docs/features/01-repositories.md");
    let named: Vec<(&str, Vec<&str>)> = found
        .iter()
        .map(|each| (each.file.as_str(), each.names.iter().map(String::as_str).collect()))
        .collect();
    assert_eq!(
        named,
        vec![
            ("test/e2e/ui.mjs", vec!["with keeping on, it shows the commit"]),
            ("test/kept.test.ts", vec![]),
            ("rust/crates/server/tests/stdio/summaries.rs", vec!["reads once"]),
        ]
    );
    assert_eq!(found[0].line, 1);
    assert_eq!(found[0].path(), "extension/test/e2e/ui.mjs");
    assert_eq!(found[2].path(), "rust/crates/server/tests/stdio/summaries.rs");
}

#[test]
fn leaves_out_none_and_what_is_not_a_file_of_tests() {
    let text =
        "- **Tested by**: `test/e2e/suite.ts`, the steps that log `> git pull --autostash`; *none* for the rest.\n";
    let found = parse_tested_by(text, "docs/features/extension/16-undo-preview-log.md");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].file, "test/e2e/suite.ts");
    assert_eq!(found[0].names, Vec::<String>::new());
}

#[test]
fn finds_a_name_at_the_start_of_a_string_written_over_lines_or_with_escaped_quotes() {
    let code =
        "await step(\n  'follows the repository selected in VS Code\\'s view,\n   and shows it', async () => {});";
    assert!(names_test(
        code,
        "follows the repository selected in VS Code's view, and shows it"
    ));
    assert!(names_test(code, "follows the repository"));
    assert!(!names_test(code, "the repository selected"));
}
