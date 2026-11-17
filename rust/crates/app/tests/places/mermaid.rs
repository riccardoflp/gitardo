//! The diagrams of Mermaid of the preview of a Markdown file, drawn from the store on repositories made for each test:
//! asked in the theme of the window, drawn on a worker once for their text and their theme, as wide as the page at
//! most, named for the screen reader, their code shown with why when they cannot be drawn, and pictures of them in each
//! theme.

use app::store::{Effect, JobKind, Message, State};
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::file_view::{DiagramShown, FilePageIntent, MarkdownIntent};
use views::text::file_view as words;

use super::file_view::{shown, text};
use super::markdown::{ask_page, click, drawn, is_preview, preview_of};
use super::pointer::rect as rect_of;
use super::showing;
use super::support::{self, Window, says};

/// The size of the window of the preview.
const SIZE: (u32, u32) = (1000, 760);

/// A flowchart with a title, a sequence diagram, a diagram with an error and one of no kind.
const DIAGRAMS: &str = "# Diagrams\n\n\
```mermaid\nflowchart LR\n  accTitle: How a commit is made\n  A[Edit files] --> B{Staged?}\n  B -->|Yes| C[Commit]\n  B -->|No| D[Stage the changes]\n  D --> C\n```\n\n\
```mermaid\nsequenceDiagram\n  participant User\n  participant Git\n  User->>Git: git commit\n  Git-->>User: Done\n```\n\n\
```mermaid\nflowchart LR\n  A[Start --> B\n```\n\n\
```mermaid\nnotADiagram\n```\n";

/// The number of diagrams of the document.
const DIAGRAM_COUNT: usize = 4;

/// A repository with the document of diagrams.
fn repository() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the diagrams", &[("DIAGRAMS.md", DIAGRAMS)]);
    repository
}

/// The preview of the diagrams, shown in a window.
fn previewing(repository: &Repository) -> State {
    let mut state = showing(repository.path().to_path_buf());
    preview_of(&mut state, "DIAGRAMS.md");
    state
}

/// Draws the window, runs what it asked, and gives the effects of that.
fn draw_and_ask(window: &mut Window, state: &mut State) -> Vec<Effect> {
    let (intents, _) = window.draw(state, &[]);
    intents
        .into_iter()
        .flat_map(|intent| support::run(state, Message::Intent(intent)))
        .collect()
}

/// How many diagrams some effects started to draw.
fn drawings(effects: &[Effect]) -> usize {
    effects
        .iter()
        .filter(|effect| matches!(effect, Effect::Start(job) if matches!(job.kind, JobKind::DrawDiagram { .. })))
        .count()
}

/// The window, its diagrams asked for and drawn, and its tree.
fn with_diagrams(window: &mut Window, state: &mut State) -> TreeUpdate {
    let _effects = draw_and_ask(window, state);
    drawn(window, state)
}

/// The bounds of the node that says a text, by its name or its value.
fn bounds_of(tree: &TreeUpdate, text: &str) -> ui::Rect {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.label() == Some(text) || node.value() == Some(text))
        .find_map(|(_, node)| node.bounds().map(rect_of))
        .unwrap()
}

#[test]
fn file_view_mermaid__each_theme_draws_the_diagrams_on_the_ground_of_the_page_with_its_text() {
    let repository = repository();
    for (theme, name) in [
        (ThemeName::Dark, "markdown-mermaid-dark"),
        (ThemeName::Light, "markdown-mermaid-light"),
        (ThemeName::HighContrastDark, "markdown-mermaid-high-contrast-dark"),
        (ThemeName::HighContrastLight, "markdown-mermaid-high-contrast-light"),
    ] {
        let mut state = previewing(&repository);
        let mut window = Window::new(theme, SIZE);
        let _tree = with_diagrams(&mut window, &mut state);
        let preview = shown(&state).markdown.as_ref().unwrap();
        assert_eq!(preview.content.diagram_theme, Some(theme));
        let drawn_count = preview
            .content
            .diagrams
            .iter()
            .filter(|(_, shown)| matches!(shown, DiagramShown::Drawn(_)))
            .count();
        assert_eq!(drawn_count, 2, "{theme:?}");
        window.assert_snapshot(name);
    }
}

#[test]
fn file_view_mermaid__an_error_and_a_text_of_no_kind_show_their_code_and_why() {
    let repository = repository();
    let mut state = previewing(&repository);
    // Tall enough for every diagram to be in sight, as only what is in sight is named for the screen reader.
    let mut window = Window::new(ThemeName::Dark, (1000, 1600));
    let tree = with_diagrams(&mut window, &mut state);
    assert!(says(&tree, "flowchart LR\n  A[Start --> B"));
    assert!(says(
        &tree,
        &words::diagram_error("Unterminated node label (missing `]`)")
    ));
    assert!(says(&tree, "notADiagram"));
    assert!(says(&tree, words::DIAGRAM_UNSUPPORTED));
    assert!(!says(
        &tree,
        "sequenceDiagram\n  participant User\n  participant Git\n  User->>Git: git commit\n  Git-->>User: Done"
    ));
}

#[test]
fn file_view_mermaid__a_diagram_wider_than_the_page_is_as_wide_as_the_page_its_height_scaled() {
    let repository = repository();
    let mut state = previewing(&repository);
    let mut window = Window::new(ThemeName::Dark, (360, 760));
    let tree = with_diagrams(&mut window, &mut state);
    let rect = bounds_of(&tree, "How a commit is made");
    let preview = shown(&state).markdown.as_ref().unwrap();
    let flowchart = preview
        .content
        .diagrams
        .iter()
        .find_map(|(source, shown)| match shown {
            DiagramShown::Drawn(diagram) if source.contains("accTitle") => Some(diagram),
            _ => None,
        })
        .unwrap();
    let natural = (f64::from(flowchart.width), f64::from(flowchart.height));
    let (width, height) = (f64::from(rect.width), f64::from(rect.height));
    assert!(natural.0 > 360.0, "{natural:?}");
    assert!(width < 360.0, "{rect:?}");
    let scaled = natural.1 * width / natural.0;
    assert!((height - scaled).abs() <= 1.0, "{rect:?} {scaled}");
}

#[test]
fn file_view_mermaid__a_diagram_is_drawn_once_for_its_text_and_its_theme() {
    let repository = repository();
    let mut state = previewing(&repository);
    let mut window = Window::new(ThemeName::Dark, SIZE);
    assert_eq!(drawings(&draw_and_ask(&mut window, &mut state)), DIAGRAM_COUNT);
    assert_eq!(drawings(&draw_and_ask(&mut window, &mut state)), 0);
    let _source = ask_page(&mut state, FilePageIntent::Markdown(MarkdownIntent::ShowSource));
    let again = ask_page(&mut state, FilePageIntent::Markdown(MarkdownIntent::ShowPreview));
    assert_eq!(drawings(&again), 0);
    let mut light = Window::new(ThemeName::Light, SIZE);
    assert_eq!(drawings(&draw_and_ask(&mut light, &mut state)), DIAGRAM_COUNT);
    assert_eq!(drawings(&draw_and_ask(&mut window, &mut state)), 0);
    let preview = shown(&state).markdown.as_ref().unwrap();
    assert_eq!(preview.content.diagram_theme, Some(ThemeName::Dark));
    assert_eq!(preview.content.diagrams.len(), DIAGRAM_COUNT);
}

#[test]
fn file_view_mermaid__the_screen_reader_reads_the_title_of_a_diagram_or_mermaid_diagram() {
    let repository = repository();
    let mut state = previewing(&repository);
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = with_diagrams(&mut window, &mut state);
    assert!(says(&tree, "How a commit is made"));
    assert!(says(&tree, words::DIAGRAM));
}

#[test]
fn file_view_mermaid__source_shows_the_block_of_the_diagram_as_its_text() {
    let repository = repository();
    let mut state = previewing(&repository);
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let _tree = with_diagrams(&mut window, &mut state);
    let _effects = click(&mut window, &mut state, (Role::Button, words::SOURCE));
    assert!(!is_preview(&state));
    assert!(text(&state).contains("```mermaid\nflowchart LR\n  accTitle: How a commit is made\n"));
    let (intents, _) = window.draw(&state, &[]);
    let asks_diagrams = intents.iter().any(|intent| {
        matches!(
            intent,
            Intent::FileView(views::file_view::FileViewIntent::Page(
                _,
                FilePageIntent::Markdown(MarkdownIntent::DrawDiagrams(_))
            ))
        )
    });
    assert!(!asks_diagrams);
}
