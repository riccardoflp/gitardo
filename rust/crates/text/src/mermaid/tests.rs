#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]

use super::*;

/// A dark look, measured as a browser measures Mermaid's own typeface.
fn look() -> Look {
    let gray = |value| Rgb {
        red: value,
        green: value,
        blue: value,
    };
    Look {
        ground: Ground::Dark,
        palette: Palette {
            background: gray(0x1e),
            text: gray(0xd4),
            line: gray(0x8a),
            border: Rgb {
                red: 0x3b,
                green: 0x8e,
                blue: 0xea,
            },
            fill: gray(0x2a),
            note: gray(0x33),
        },
        font: Font {
            family: String::from("Inter"),
            size: 14.0,
            width: None,
        },
    }
}

#[test]
fn file_view_mermaid__a_flowchart_and_a_sequence_diagram_are_drawn_with_their_labels() {
    let flowchart = diagram("flowchart LR\n  A[Edit files] --> B[Commit]\n", &look()).unwrap();
    assert!(flowchart.svg.starts_with("<svg"), "{}", flowchart.svg);
    for label in ["Edit files", "Commit"] {
        assert!(flowchart.svg.contains(label), "{label}");
    }
    let sequence = diagram(
        "sequenceDiagram\n  participant User\n  participant Git\n  User->>Git: git commit\n",
        &look(),
    )
    .unwrap();
    for label in ["User", "Git", "git commit"] {
        assert!(sequence.svg.contains(label), "{label}");
    }
}

#[test]
fn file_view_mermaid__links_clicks_html_images_and_scripts_reach_nothing() {
    let source = "flowchart LR\n  A[\"<b>bold</b> <img src='https://example.com/x.png'> <script>alert(1)</script>\"] --> B\n\
                  click A \"javascript:alert(1)\"\n  click B callback\n";
    let drawn = diagram(source, &look()).unwrap();
    for absent in [
        "<foreignObject",
        "<script",
        "<img",
        "<image",
        "javascript:",
        "onclick",
        "example.com/x.png",
    ] {
        assert!(!drawn.svg.contains(absent), "{absent} in {}", drawn.svg);
    }
    assert!(drawn.svg.contains("bold"));
}

#[test]
fn an_error_is_said_in_the_words_of_the_library_without_the_kind_of_diagram() {
    let invalid = diagram("flowchart LR\n  A[Start --> B\n", &look());
    let Err(NotDrawn::Invalid(reason)) = invalid else {
        panic!("{invalid:?}");
    };
    assert!(reason.starts_with("Unterminated node label"), "{reason}");
    assert_eq!(diagram("notADiagram\n  A --> B\n", &look()), Err(NotDrawn::Unsupported));
    assert_eq!(diagram("", &look()), Err(NotDrawn::Unsupported));
}

#[test]
fn the_title_and_the_description_are_read_from_acc_title_and_acc_descr() {
    let source = "flowchart LR\n  accTitle: How a commit is made\n  accDescr: Staged &amp; committed\n  A --> B\n";
    let drawn = diagram(source, &look()).unwrap();
    assert_eq!(drawn.title.as_deref(), Some("How a commit is made"));
    assert!(drawn.description.unwrap().starts_with("Staged"));
    let untitled = diagram("flowchart LR\n  A --> B\n", &look()).unwrap();
    assert_eq!(untitled.title, None);
}

#[test]
fn the_colors_and_the_typeface_of_the_look_are_those_of_the_svg() {
    let drawn = diagram("flowchart LR\n  A --> B\n", &look()).unwrap();
    for said in ["#d4d4d4", "#3b8eea", "Inter"] {
        assert!(drawn.svg.contains(said), "{said}");
    }
}

#[test]
fn a_typeface_given_measures_the_labels() {
    let wide = Look {
        font: Font {
            width: Some(Arc::new(|text: &str, size, _| {
                f64::from(u32::try_from(text.chars().count()).unwrap()) * size * 2.0
            })),
            ..look().font
        },
        ..look()
    };
    let narrow = diagram("flowchart LR\n  A[Some words]\n", &look()).unwrap();
    let measured = diagram("flowchart LR\n  A[Some words]\n", &wide).unwrap();
    let width = |svg: &str| {
        let start = svg.find("viewBox=\"").unwrap() + 9;
        let numbers: Vec<f64> = svg[start..]
            .split('"')
            .next()
            .unwrap()
            .split(' ')
            .map(|n| n.parse().unwrap())
            .collect();
        numbers[2]
    };
    assert!(
        width(&measured.svg) > width(&narrow.svg),
        "{} {}",
        width(&measured.svg),
        width(&narrow.svg)
    );
}

#[test]
fn entities_of_the_title_are_read() {
    assert_eq!(
        accessible::text_of("<svg><title id=\"t\">A &lt;b&gt; &#38; &#x41;</title></svg>", "title").as_deref(),
        Some("A <b> & A")
    );
    assert_eq!(accessible::text_of("<svg><titles>x</titles></svg>", "title"), None);
    assert_eq!(accessible::text_of("<svg><desc> </desc></svg>", "desc"), None);
}

#[test]
fn a_diagram_that_asks_for_the_elk_layout_is_drawn_with_the_default_one() {
    let source = "---\nconfig:\n  layout: elk\n---\nflowchart LR\n  A[Edit files] --> B[Commit]\n";
    let drawn = diagram(source, &look());
    assert!(
        drawn.as_ref().is_ok_and(|drawn| drawn.svg.contains("Edit files")),
        "{drawn:?}"
    );
}
