use super::*;

/// A span of plain text.
fn plain(text: &str) -> Piece {
    Piece::Text(Span {
        text: text.to_owned(),
        look: Look::default(),
        link: None,
    })
}

/// A span of text in a look.
fn looking(text: &str, look: Look) -> Piece {
    Piece::Text(Span {
        text: text.to_owned(),
        look,
        link: None,
    })
}

#[test]
fn headings_paragraphs_and_their_anchors_as_github_names_them() {
    let read = blocks("# Getting started!\n\nSome text\nover two lines.\n\n## Getting started\n");
    assert_eq!(
        read,
        vec![
            Block::Heading {
                level: 1,
                text: vec![plain("Getting started!")],
                anchor: String::from("getting-started"),
            },
            Block::Paragraph(vec![plain("Some text over two lines.")]),
            Block::Heading {
                level: 2,
                text: vec![plain("Getting started")],
                anchor: String::from("getting-started-1"),
            },
        ]
    );
}

#[test]
fn emphasis_strong_strikethrough_code_and_links_are_spans_of_their_look() {
    let read = blocks("A **bold** *soft* ~~gone~~ `code` [link](https://example.com).");
    let strong = Look {
        is_strong: true,
        ..Look::default()
    };
    let emphasis = Look {
        is_emphasis: true,
        ..Look::default()
    };
    let struck = Look {
        is_struck: true,
        ..Look::default()
    };
    let code = Look {
        is_code: true,
        ..Look::default()
    };
    let link = Piece::Text(Span {
        text: String::from("link"),
        look: Look::default(),
        link: Some(String::from("https://example.com")),
    });
    assert_eq!(
        read,
        vec![Block::Paragraph(vec![
            plain("A "),
            looking("bold", strong),
            plain(" "),
            looking("soft", emphasis),
            plain(" "),
            looking("gone", struck),
            plain(" "),
            looking("code", code),
            plain(" "),
            link,
            plain("."),
        ])]
    );
}

#[test]
fn lists_of_bullets_numbers_and_tasks_with_a_list_inside_an_item() {
    let read = blocks("- one\n  - inner\n- two\n\n3. three\n4. four\n\n- [x] done\n- [ ] to do\n");
    let item = |mark, text: &str| Item {
        mark,
        blocks: vec![Block::Paragraph(vec![plain(text)])],
    };
    let first = Item {
        mark: Mark::Bullet,
        blocks: vec![
            Block::Paragraph(vec![plain("one")]),
            Block::List(vec![item(Mark::Bullet, "inner")]),
        ],
    };
    assert_eq!(
        read,
        vec![
            Block::List(vec![first, item(Mark::Bullet, "two")]),
            Block::List(vec![item(Mark::Number(3), "three"), item(Mark::Number(4), "four")]),
            Block::List(vec![
                item(Mark::Task { is_done: true }, "done"),
                item(Mark::Task { is_done: false }, "to do"),
            ]),
        ]
    );
}

#[test]
fn a_table_keeps_the_alignment_of_its_columns_and_its_cells() {
    let read = blocks("| Name | Count | Note |\n| :--- | ---: | :-: |\n| a | 1 | x |\n| b | 2 | y |\n");
    assert_eq!(
        read,
        vec![Block::Table(Table {
            alignments: vec![Alignment::Start, Alignment::End, Alignment::Center],
            head: vec![vec![plain("Name")], vec![plain("Count")], vec![plain("Note")]],
            rows: vec![
                vec![vec![plain("a")], vec![plain("1")], vec![plain("x")]],
                vec![vec![plain("b")], vec![plain("2")], vec![plain("y")]],
            ],
        })]
    );
}

#[test]
fn code_blocks_quotes_rules_and_html_shown_as_its_text() {
    let read = blocks("```rust\nfn main() {}\n```\n\n> quoted\n\n---\n\n<script>alert(1)</script>\n");
    assert_eq!(
        read,
        vec![
            Block::Code(CodeBlock {
                language: String::from("rust"),
                text: String::from("fn main() {}"),
            }),
            Block::Quote(vec![Block::Paragraph(vec![plain("quoted")])]),
            Block::Rule,
            Block::Code(CodeBlock {
                language: String::from("html"),
                text: String::from("<script>alert(1)</script>"),
            }),
        ]
    );
}

#[test]
fn inline_html_is_text_and_an_image_keeps_its_source_its_text_and_its_link() {
    let read = blocks("Say <b>hi</b> [![logo](media/logo.png)](https://example.com)");
    assert_eq!(
        read,
        vec![Block::Paragraph(vec![
            plain("Say <b>hi</b> "),
            Piece::Image {
                source: String::from("media/logo.png"),
                text: String::from("logo"),
                link: Some(String::from("https://example.com")),
            },
        ])]
    );
}

#[test]
fn links_go_to_the_web_to_files_relative_to_the_file_and_to_headings() {
    let cases = [
        (
            "https://example.com",
            LinkTarget::Web(String::from("https://example.com")),
        ),
        (
            "mailto:someone@example.com",
            LinkTarget::Web(String::from("mailto:someone@example.com")),
        ),
        (
            "../CHANGELOG.md",
            LinkTarget::File {
                path: String::from("CHANGELOG.md"),
                line: None,
            },
        ),
        (
            "guide%20two.md#L12",
            LinkTarget::File {
                path: String::from("docs/guide two.md"),
                line: Some(12),
            },
        ),
        (
            "/src/app.rs",
            LinkTarget::File {
                path: String::from("src/app.rs"),
                line: None,
            },
        ),
        ("#Getting-Started", LinkTarget::Heading(String::from("getting-started"))),
        ("javascript:alert(1)", LinkTarget::Nowhere),
        ("../../outside.md", LinkTarget::Nowhere),
    ];
    for (href, expected) in cases {
        assert_eq!(link_target("docs/guide.md", href), expected, "{href}");
    }
}

#[test]
fn an_image_of_the_web_has_no_path_and_one_of_the_repository_has_its_path() {
    assert_eq!(image_path("README.md", "https://example.com/a.png"), None);
    assert_eq!(
        image_path("docs/guide.md", "../media/logo.png"),
        Some(String::from("media/logo.png"))
    );
}

#[test]
fn the_language_of_a_fence_names_a_file_whose_colors_it_takes() {
    let cases = [
        ("rust", "code.rs"),
        ("ts", "code.ts"),
        ("Python", "code.py"),
        ("bash", "code.sh"),
        ("", "code."),
    ];
    for (language, expected) in cases {
        assert_eq!(code_file_name(language), expected, "{language}");
    }
}

#[test]
fn markdown_files_are_told_by_the_end_of_their_name() {
    let cases = [
        ("README.md", true),
        ("docs/guide.MARKDOWN", true),
        ("src/app.rs", false),
        (".md", false),
        ("md", false),
    ];
    for (path, expected) in cases {
        assert_eq!(is_markdown(path), expected, "{path}");
    }
}

#[test]
fn code_blocks_and_images_are_found_inside_lists_quotes_and_tables() {
    let read = blocks(
        "- item\n\n  ```rust\n  fn a() {}\n  ```\n\n> ```\n> b\n> ```\n\n| ![one](a.png) |\n| - |\n| ![two](b.png) ![one](a.png) |\n",
    );
    let codes: Vec<&str> = code_blocks(&read).iter().map(|code| code.text.as_str()).collect();
    assert_eq!(codes, vec!["fn a() {}", "b"]);
    assert_eq!(image_sources(&read), vec!["a.png", "b.png"]);
}

#[test]
fn the_diagrams_of_mermaid_are_the_blocks_fenced_as_mermaid_each_once() {
    let read = blocks(
        "```mermaid\nflowchart LR\n  A --> B\n```\n\n> ```Mermaid\n> pie\n> ```\n\n```mermaid\nflowchart LR\n  A --> B\n```\n\n```rust\nfn a() {}\n```\n",
    );
    assert_eq!(mermaid_sources(&read), vec!["flowchart LR\n  A --> B", "pie"]);
}
