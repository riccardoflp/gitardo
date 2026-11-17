//! The preview of a Markdown file in the File page, drawn from the store, on repositories made for each test: its
//! blocks laid out, **Source** and **Preview**, the setting that opens it, HTML shown as text, its links and its images,
//! and the preview drawn in each theme.

use app::store::{Effect, Message, State};
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use testkit::png;
use ui::{CodeKind, Modifiers, NamedKey};
use views::Intent;
use views::file_view::{FilePageIntent, FileTarget, FileViewIntent, MarkdownIntent};
use views::files::FilesPageIntent;
use views::search::QuickCommand;
use views::text::file_view as words;

use super::blame::quick;
use super::file_view::{key, open, shown, step, text, type_text};
use super::pointer::{middle, rect as rect_of};
use super::showing;
use super::support::{self, OperationsDriver, Window, has, says, says_in};

/// The size of the window of the preview.
const SIZE: (u32, u32) = (1000, 700);

/// A README with a heading, a list, a task list, a table, a block of Rust, a quote and a link.
const README: &str = "# Gitardo\n\nA *window* for **Git**, with `code` and ~~less~~ [a guide](docs/guide.md).\n\n\
- Fast\n- Small\n\n- [x] Read\n- [ ] Write\n\n| Name | Count |\n| :--- | ---: |\n| main | 12 |\n| next | 3 |\n\n\
```rust\nfn main() {\n    println!(\"hi\");\n}\n```\n\n> Quoted words.\n";

/// A repository with a README and a guide in `docs`, linking to the CHANGELOG and to the web.
fn repository() -> Repository {
    let mut repository = Repository::new();
    let guide = "# Guide\n\nSee [the changes](../CHANGELOG.md) and [the site](https://example.com).\n\n\
<script>alert(1)</script>\n\n## Usage\n\nBack to [the top](#guide).\n";
    let _ = repository.commit(
        "Add the documents",
        &[
            ("README.md", README),
            ("docs/guide.md", guide),
            ("CHANGELOG.md", "# Changelog\n"),
            ("src/app.rs", "fn main() {}\n"),
        ],
    );
    repository
}

/// Asks something of the page shown.
fn ask_page(state: &mut State, intent: FilePageIntent) -> Vec<Effect> {
    let id = shown(state).id;
    support::run(
        state,
        Message::Intent(Intent::FileView(FileViewIntent::Page(id, intent))),
    )
}

/// Opens a file of the working tree in its page, then shows its preview.
fn preview_of(state: &mut State, path: &str) {
    open(state, FileTarget::working_tree(path));
    let _effects = ask_page(state, FilePageIntent::Markdown(MarkdownIntent::ShowPreview));
}

/// Whether the page shown shows its preview.
fn is_preview(state: &State) -> bool {
    shown(state).markdown.as_ref().is_some_and(|preview| preview.is_shown)
}

/// Clicks the widget of a role and a name, and gives what the click asked, run.
fn click(window: &mut Window, state: &mut State, (role, name): (Role, &str)) -> Vec<Effect> {
    let intents = window.click(state, role, name);
    intents
        .into_iter()
        .flat_map(|intent| support::run(state, Message::Intent(intent)))
        .collect()
}

/// The window drawn once, and its tree.
fn drawn(window: &mut Window, state: &State) -> TreeUpdate {
    window.draw(state, &[]).1
}

#[test]
fn file_view_markdown_preview__readme_shows_its_heading_list_table_and_rust_code_laid_out_the_code_in_rust_colors() {
    let repository = repository();
    let mut state = showing(repository.path().to_path_buf());
    preview_of(&mut state, "README.md");
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = drawn(&mut window, &state);
    assert!(has(&tree, Role::Heading, "Gitardo"));
    for said in ["Fast", "Small", "Read", "main", "12", "Quoted words."] {
        assert!(says(&tree, said), "{said}");
    }
    assert!(says(&tree, "fn main() {\n    println!(\"hi\");\n}"));
    let preview = shown(&state).markdown.as_ref().unwrap();
    let first_line = &preview.content.code_colors[0][0];
    assert!(
        first_line
            .iter()
            .any(|span| span.kind == CodeKind::Keyword && span.range == (0..2))
    );
}

#[test]
fn file_view_markdown_preview__source_shows_the_text_editable_and_preview_shows_it_again_with_the_edit() {
    let repository = repository();
    let mut state = showing(repository.path().to_path_buf());
    preview_of(&mut state, "README.md");
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let _effects = click(&mut window, &mut state, (Role::Button, words::SOURCE));
    assert!(!is_preview(&state));
    let _ = step(&mut window, &mut state, &[]);
    type_text(&mut window, &mut state, "Hello");
    let enter = key(NamedKey::Enter, Modifiers::NONE);
    let _ = step(&mut window, &mut state, &[enter.clone(), enter]);
    assert!(text(&state).starts_with("Hello\n\n# Gitardo"), "{}", text(&state));
    let _effects = click(&mut window, &mut state, (Role::Button, words::PREVIEW));
    assert!(is_preview(&state));
    let tree = drawn(&mut window, &state);
    assert!(says(&tree, "Hello"));
    assert!(has(&tree, Role::Heading, "Gitardo"));
}

#[test]
fn file_view_markdown_preview__with_the_setting_on_a_file_opened_from_the_files_page_opens_in_preview_off_in_source() {
    for is_on in [true, false] {
        let repository = repository();
        let mut state = showing(repository.path().to_path_buf());
        state.settings.markdown_open_as_preview = is_on;
        let driver = OperationsDriver::new();
        quick(&driver, &mut state, QuickCommand::ShowFiles);
        let fold = Intent::Files(FilesPageIntent::Fold(String::from("docs")));
        let _effects = support::run(&mut state, Message::Intent(fold));
        let mut window = Window::new(ThemeName::Dark, SIZE);
        let tree = drawn(&mut window, &state);
        let row = tree
            .nodes
            .iter()
            .filter(|(_, node)| node.value() == Some("guide.md") || node.label() == Some("guide.md"))
            .filter_map(|(_, node)| node.bounds().map(rect_of))
            .max_by(|first, second| first.y.total_cmp(&second.y));
        let point = middle(row.unwrap());
        let intents = super::pointer::click_at(&mut window, &state, point, 1);
        for intent in intents {
            let _effects = support::run(&mut state, Message::Intent(intent));
        }
        assert_eq!(shown(&state).path, "docs/guide.md");
        assert_eq!(is_preview(&state), is_on);
        let tree = drawn(&mut window, &state);
        assert_eq!(has(&tree, Role::Heading, "Guide"), is_on);
    }
}

#[test]
fn file_view_markdown_preview__a_script_in_the_file_is_shown_as_that_text() {
    let repository = repository();
    let mut state = showing(repository.path().to_path_buf());
    preview_of(&mut state, "docs/guide.md");
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = drawn(&mut window, &state);
    assert!(says(&tree, "<script>alert(1)</script>"));
}

#[test]
fn file_view_markdown_preview__a_link_to_the_changelog_opens_its_page_and_a_link_to_the_web_asks_the_browser() {
    let repository = repository();
    let mut state = showing(repository.path().to_path_buf());
    preview_of(&mut state, "docs/guide.md");
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let effects = click(&mut window, &mut state, (Role::Button, "the site"));
    assert!(effects.contains(&Effect::OpenUrl(String::from("https://example.com"))));
    let _effects = click(&mut window, &mut state, (Role::Button, "the changes"));
    assert_eq!(shown(&state).path, "CHANGELOG.md");
}

#[test]
fn file_view_markdown_preview__a_link_to_a_heading_scrolls_to_it() {
    let repository = repository();
    let mut state = showing(repository.path().to_path_buf());
    preview_of(&mut state, "docs/guide.md");
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let _effects = click(&mut window, &mut state, (Role::Button, "the top"));
    let preview = shown(&state).markdown.as_ref().unwrap();
    assert_eq!(
        preview.content.reveal.as_ref().map(|(_, anchor)| anchor.as_str()),
        Some("guide")
    );
}

#[test]
fn file_view_markdown_preview__an_image_of_the_repository_is_drawn_and_one_of_the_web_shows_its_text() {
    let repository = repository();
    let logo = png::Image::filled(40, 20, [200, 40, 40, 255]);
    let media = repository.path().join("media");
    std::fs::create_dir_all(&media).unwrap();
    std::fs::write(media.join("logo.png"), png::encode(&logo).unwrap()).unwrap();
    std::fs::write(
        repository.path().join("IMAGES.md"),
        "![Logo](media/logo.png) ![Remote](https://example.com/a.png)\n",
    )
    .unwrap();
    let mut state = showing(repository.path().to_path_buf());
    preview_of(&mut state, "IMAGES.md");
    let preview = shown(&state).markdown.as_ref().unwrap();
    let sources: Vec<&str> = preview
        .content
        .images
        .iter()
        .map(|(source, _)| source.as_str())
        .collect();
    assert_eq!(sources, vec!["media/logo.png"]);
    assert_eq!(preview.content.images[0].1.width(), 40);
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = drawn(&mut window, &state);
    assert!(says_in(&tree, "Remote"));
}

#[test]
fn file_view_markdown_preview__src_app_rs_has_no_preview() {
    let repository = repository();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, FileTarget::working_tree("src/app.rs"));
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = drawn(&mut window, &state);
    assert!(!has(&tree, Role::Button, words::PREVIEW));
    assert!(shown(&state).markdown.is_none());
    open(&mut state, FileTarget::working_tree("README.md"));
    let tree = drawn(&mut window, &state);
    assert!(has(&tree, Role::Button, words::PREVIEW));
}

#[test]
fn the_preview_of_a_markdown_file_is_drawn_in_each_theme() {
    let repository = repository();
    for (theme, name) in [
        (ThemeName::Dark, "markdown-preview-dark"),
        (ThemeName::Light, "markdown-preview-light"),
        (ThemeName::HighContrastDark, "markdown-preview-high-contrast-dark"),
        (ThemeName::HighContrastLight, "markdown-preview-high-contrast-light"),
    ] {
        let mut state = showing(repository.path().to_path_buf());
        preview_of(&mut state, "README.md");
        let mut window = Window::new(theme, SIZE);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(name);
    }
}
