use super::*;

/// The tokens of a text, as the text of each and its kind, line after line.
fn tokens(path: &str, text: &str) -> Vec<Vec<(String, TokenKind)>> {
    let lines: Vec<&str> = text.split('\n').collect();
    highlight(&HighlightInput { path, text })
        .iter()
        .zip(lines)
        .map(|(line, text)| {
            line.tokens
                .iter()
                .map(|token| (text[token.range.clone()].to_owned(), token.kind))
                .collect()
        })
        .collect()
}

/// The kind of the first token of a text, as it is cut in a file.
fn kind_of(path: &str, text: &str, token: &str) -> Option<TokenKind> {
    tokens(path, text)
        .into_iter()
        .flatten()
        .find(|(found, _)| found == token)
        .map(|(_, kind)| kind)
}

#[test]
fn file_view_highlight__fn_a_string_a_comment_and_a_number_of_rust_have_their_kinds() {
    let text = "fn main() {\n    let answer = 42; // note\n    println!(\"text\");\n}\n";
    assert_eq!(kind_of("src/main.rs", text, "fn"), Some(TokenKind::Keyword));
    assert_eq!(kind_of("src/main.rs", text, "\"text\""), Some(TokenKind::String));
    assert_eq!(kind_of("src/main.rs", text, "// note"), Some(TokenKind::Comment));
    assert_eq!(kind_of("src/main.rs", text, "42"), Some(TokenKind::Number));
    assert_eq!(kind_of("src/main.rs", text, "main"), Some(TokenKind::Function));
    assert_eq!(kind_of("src/main.rs", text, "println"), Some(TokenKind::Function));
    assert_eq!(kind_of("src/main.rs", text, "answer"), Some(TokenKind::Variable));
}

#[test]
fn the_language_is_told_by_the_name_of_the_file() {
    let cases = [
        ("src/app.tsx", Some(Language::TypeScript)),
        ("lib/index.cjs", Some(Language::JavaScript)),
        ("settings.jsonc", Some(Language::Json)),
        ("pages/[id].vue", Some(Language::Markup)),
        ("icon.svg", Some(Language::Markup)),
        ("build.gradle.kts", Some(Language::Kotlin)),
        ("include/list.hh", Some(Language::Cpp)),
        ("Dockerfile", Some(Language::Dockerfile)),
        ("tools/web.dockerfile", Some(Language::Dockerfile)),
        ("setup.PS1", Some(Language::PowerShell)),
        ("README", None),
        ("notes.txt", None),
    ];
    for (path, language) in cases {
        assert_eq!(Language::of(path), language, "{path}");
    }
}

#[test]
fn a_file_of_no_language_has_no_tokens_but_one_entry_per_line() {
    let lines = highlight(&HighlightInput {
        path: "notes.txt",
        text: "fn 42\n\"a\"\n",
    });
    assert_eq!(lines.len(), 3);
    assert!(lines.iter().all(|line| line.tokens.is_empty()));
}

#[test]
fn a_block_comment_and_a_string_over_lines_are_colored_as_one() {
    let lines = tokens("a.ts", "let a = 1; /* one\ntwo\nthree */ let b = `x\ny`;");
    assert_eq!(lines[1], [("two".to_owned(), TokenKind::Comment)]);
    assert_eq!(lines[2][0], ("three */".to_owned(), TokenKind::Comment));
    assert_eq!(lines[2][1], ("let".to_owned(), TokenKind::Keyword));
    assert_eq!(lines[3][0], ("y`".to_owned(), TokenKind::String));
    let python = tokens("a.py", "x = \"\"\"doc\nstill doc\n\"\"\"\ndef f(): pass");
    assert_eq!(python[1], [("still doc".to_owned(), TokenKind::String)]);
    assert_eq!(python[3][0], ("def".to_owned(), TokenKind::Keyword));
}

#[test]
fn a_string_on_one_line_ends_with_it_when_it_is_not_closed() {
    let lines = tokens("a.c", "char *s = \"open\nint x;");
    assert_eq!(lines[1][0], ("int".to_owned(), TokenKind::Type));
}

#[test]
fn names_are_told_apart_by_what_is_around_them() {
    let text = "let user = User::new(); user.name.len(); const MAX_SIZE: usize = 'a' as usize; 'outer: loop {}";
    assert_eq!(kind_of("a.rs", text, "User"), Some(TokenKind::Type));
    assert_eq!(kind_of("a.rs", text, "new"), Some(TokenKind::Function));
    assert_eq!(kind_of("a.rs", text, "name"), Some(TokenKind::Property));
    assert_eq!(kind_of("a.rs", text, "MAX_SIZE"), Some(TokenKind::Constant));
    assert_eq!(kind_of("a.rs", text, "usize"), Some(TokenKind::Type));
    assert_eq!(kind_of("a.rs", text, "'a'"), Some(TokenKind::String));
    assert_eq!(kind_of("a.rs", text, "'outer"), Some(TokenKind::Type));
    assert_eq!(
        kind_of("a.rs", "#[derive(Debug)] struct A;", "#[derive(Debug)]"),
        Some(TokenKind::Attribute)
    );
    assert_eq!(
        kind_of("a.rs", "r#\"raw \" text\"#", "r#\"raw \" text\"#"),
        Some(TokenKind::String)
    );
    assert_eq!(kind_of("a.rs", "0..10", "0"), Some(TokenKind::Number));
    assert_eq!(kind_of("a.rs", "x == 2.5e3", "2.5e3"), Some(TokenKind::Number));
}

#[test]
fn keys_marks_and_sections_have_their_kinds() {
    let cases = [
        ("a.json", "{\"name\": \"Ada\"}", "\"name\"", TokenKind::Property),
        ("a.json", "{\"name\": \"Ada\"}", "\"Ada\"", TokenKind::String),
        ("a.yml", "runs-on: ubuntu-latest", "runs-on", TokenKind::Property),
        ("a.yml", "runs-on: ubuntu-latest", "ubuntu-latest", TokenKind::String),
        (
            "a.toml",
            "[package]
name = \"gitardo\"",
            "[package]",
            TokenKind::Type,
        ),
        (
            "a.toml",
            "[package]
name = \"gitardo\"",
            "name",
            TokenKind::Property,
        ),
        ("a.sh", "echo \"$HOME\" ${PATH} # done", "${PATH}", TokenKind::Variable),
        ("a.sh", "echo \"$HOME\" ${PATH} # done", "# done", TokenKind::Comment),
        ("a.ps1", "if ($true) { Get-ChildItem }", "$true", TokenKind::Constant),
        (
            "a.ps1",
            "if ($true) { Get-ChildItem }",
            "Get-ChildItem",
            TokenKind::Function,
        ),
        ("a.sql", "SELECT name FROM users", "SELECT", TokenKind::Keyword),
        ("Dockerfile", "FROM rust:1 AS build", "FROM", TokenKind::Keyword),
        ("a.css", "a { margin-top: 0 }", "margin-top", TokenKind::Property),
        ("a.java", "@Override void run()", "@Override", TokenKind::Attribute),
        ("a.py", "print(f\"{x}\")", "f\"{x}\"", TokenKind::String),
        (
            "a.lua",
            "--[[ a
b ]] local x",
            "b ]]",
            TokenKind::Comment,
        ),
    ];
    for (path, text, token, kind) in cases {
        assert_eq!(kind_of(path, text, token), Some(kind), "{token} in {path}");
    }
}

#[test]
fn markup_has_tags_attributes_values_entities_and_comments() {
    let lines = tokens(
        "a.html",
        "<a href=\"/x\" class='y\nz'>Ada &amp; Test</a> <!-- note\n-->",
    );
    assert_eq!(
        lines[0][..4],
        [
            ("<".to_owned(), TokenKind::Punctuation),
            ("a".to_owned(), TokenKind::Tag),
            ("href".to_owned(), TokenKind::Attribute),
            ("=".to_owned(), TokenKind::Operator),
        ]
    );
    assert_eq!(lines[1][0], ("z'".to_owned(), TokenKind::String));
    assert!(lines[1].contains(&("&amp;".to_owned(), TokenKind::Constant)));
    assert!(lines[1].contains(&("</".to_owned(), TokenKind::Punctuation)));
    assert_eq!(lines[2], [("-->".to_owned(), TokenKind::Comment)]);
}

#[test]
fn markdown_has_headings_code_quotes_lists_and_links() {
    let lines = tokens(
        "README.md",
        "# Gitardo\n- see [the docs](docs/README.md) and `cargo test`\n```\nfn main() {}\n```\n> quoted",
    );
    assert_eq!(lines[0], [("# Gitardo".to_owned(), TokenKind::Tag)]);
    assert_eq!(
        lines[1],
        [
            ("-".to_owned(), TokenKind::Punctuation),
            ("[the docs]".to_owned(), TokenKind::Attribute),
            ("(docs/README.md)".to_owned(), TokenKind::String),
            ("`cargo test`".to_owned(), TokenKind::String),
        ]
    );
    assert_eq!(lines[3], [("fn main() {}".to_owned(), TokenKind::String)]);
    assert_eq!(lines[5], [("> quoted".to_owned(), TokenKind::Comment)]);
}

#[test]
fn coloring_again_after_an_edit_matches_coloring_the_whole_text() {
    let before = "let a = 1;\nlet b = 2;\nlet c = 3;\nlet d = 4;";
    let cases = [
        (
            "let a = 1;\nlet b = 2; /* open\nlet c = 3;\nlet d = 4;",
            (1, 1, 1),
            1..4,
        ),
        ("let a = 1;\nlet b = 2;\nlet d = 4;", (2, 1, 0), 2..2),
        ("let a = 1;\nx\ny\nlet b = 2;\nlet c = 3;\nlet d = 4;", (1, 0, 2), 1..3),
        ("`\nlet b = 2;\nlet c = 3;\nlet d = 4;", (0, 1, 1), 0..4),
    ];
    for (after, (start, removed, added), lines_colored) in cases {
        let mut colored = highlight(&HighlightInput {
            path: "a.ts",
            text: before,
        });
        let lines: Vec<String> = after.split('\n').map(str::to_owned).collect();
        let line_of = |line: usize| lines.get(line).cloned().unwrap_or_default();
        let replaced = LinesReplaced { start, removed, added };
        let recolored = highlight_again((Language::of("a.ts"), &line_of), &mut colored, replaced);
        assert_eq!(recolored, lines_colored, "{after}");
        let whole = highlight(&HighlightInput {
            path: "a.ts",
            text: after,
        });
        assert_eq!(colored, whole, "{after}");
    }
}

#[test]
fn a_line_ending_with_a_carriage_return_has_no_token_for_it() {
    let lines = highlight(&HighlightInput {
        path: "a.rs",
        text: "fn a() {}\r\nlet\r\n",
    });
    assert_eq!(
        lines[1].tokens,
        [Token {
            range: 0..3,
            kind: TokenKind::Keyword
        }]
    );
}
