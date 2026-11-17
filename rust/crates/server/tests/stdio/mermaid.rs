//! Milestone M17, `file-view.mermaid` in the extension: a diagram of Mermaid asked with no repository, written as an
//! SVG in the colors of a kind of theme of VS Code, or why it is not.

use serde_json::json;

use crate::harness::Engine;

/// A flowchart with a title.
const FLOWCHART: &str = "flowchart LR\n  accTitle: How a commit is made\n  A[Edit files] --> B[Commit]\n";

#[test]
fn vs_codes_preview_gets_the_svg_of_a_diagram_in_the_kind_of_its_theme_or_why_not() {
    let mut engine = Engine::start();
    let _ = engine.initialize();
    let dark = engine.ask(
        "read/file-view.mermaid",
        &json!({ "source": FLOWCHART, "theme": "dark" }),
    );
    let drawn = &dark["result"];
    assert_eq!(drawn["kind"], "drawn", "{dark}");
    assert_eq!(drawn["title"], "How a commit is made");
    let svg = drawn["svg"].as_str().unwrap();
    assert!(
        svg.starts_with("<svg") && svg.contains("Edit files") && svg.contains("#1f1f1f"),
        "{svg}"
    );
    assert!(drawn["width"].as_f64().unwrap() > drawn["height"].as_f64().unwrap());
    let light = engine.ask(
        "read/file-view.mermaid",
        &json!({ "source": FLOWCHART, "theme": "highContrastLight" }),
    );
    assert!(light["result"]["svg"].as_str().unwrap().contains("#0f4a85"), "{light}");
    let invalid = engine.ask(
        "read/file-view.mermaid",
        &json!({ "source": "flowchart LR\n  A[Start --> B\n", "theme": "light" }),
    );
    assert_eq!(invalid["result"]["kind"], "invalid", "{invalid}");
    assert!(invalid["result"]["reason"].as_str().unwrap().contains("Unterminated"));
    let unsupported = engine.ask(
        "read/file-view.mermaid",
        &json!({ "source": "notADiagram\n", "theme": "dark" }),
    );
    assert_eq!(unsupported["result"], json!({ "kind": "unsupported" }));
}
