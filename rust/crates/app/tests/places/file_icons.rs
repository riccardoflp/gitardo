//! The icons of Symbols beside the names of files and folders in the tree of the Files page, in each theme and in the
//! color of the text (`file-view.file-icons`).

use render::ThemeName;
use testkit::Repository;
use ui::FileIconStyle;

use super::files_page::files_page;
use super::support::{Window, says};

/// A repository whose files and folders have icons of their own: Rust and its manifest, a package of Node, a
/// Dockerfile, a README, the folders of the sources, of the documents and of GitHub, and a file nothing names.
fn varied() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit(
        "Lay out the project",
        &[
            (".github/workflows/ci.yml", "on: push\n"),
            ("assets/logo.svg", "<svg/>\n"),
            ("docs/guide.md", "# Guide\n"),
            ("src/main.rs", "fn main() {}\n"),
            ("src/index.ts", "export {};\n"),
            ("src/style.css", "body {}\n"),
            ("tests/app.test.js", "test();\n"),
            ("Cargo.toml", "[package]\n"),
            ("Dockerfile", "FROM scratch\n"),
            ("README.md", "# Project\n"),
            ("notes.xyz", "notes\n"),
            ("package.json", "{}\n"),
        ],
    );
    repository
}

/// The Files page of the varied repository with `src` and `docs` open, drawn in a theme and a style of the icons.
fn drawn(theme: ThemeName, style: FileIconStyle) -> (Window, platform::accesskit::TreeUpdate) {
    let repository = varied();
    let (mut state, _window, _driver) = files_page(&repository, &["src", "docs"]);
    state.settings.file_icons = style;
    let mut window = Window::new(theme, (640, 580));
    let _ = window.draw(&state, &[]);
    let (_, tree) = window.draw(&state, &[]);
    (window, tree)
}

#[test]
fn file_view_file_icons__the_files_page_shows_the_icon_of_each_file_and_folder_in_each_theme() {
    for (theme, name) in [
        (ThemeName::Dark, "file-icons-dark"),
        (ThemeName::Light, "file-icons-light"),
        (ThemeName::HighContrastDark, "file-icons-high-contrast-dark"),
        (ThemeName::HighContrastLight, "file-icons-high-contrast-light"),
    ] {
        let (window, _) = drawn(theme, FileIconStyle::Symbols);
        window.assert_snapshot(name);
    }
}

#[test]
fn file_view_file_icons__with_monochrome_every_icon_is_drawn_in_the_color_of_the_text() {
    let (window, _) = drawn(ThemeName::Dark, FileIconStyle::Monochrome);
    window.assert_snapshot("file-icons-monochrome-dark");
}

#[test]
fn file_view_file_icons__the_screen_reader_hears_the_name_of_a_row_and_nothing_of_its_icon() {
    let (_, tree) = drawn(ThemeName::Dark, FileIconStyle::Symbols);
    for name in ["main.rs", "Cargo.toml", "src", "docs"] {
        assert!(says(&tree, name), "{name}");
    }
    let icons = ["rust", "folder-orange-code", "folder-documents", "document"];
    assert!(tree.nodes.iter().all(|(_, node)| {
        icons
            .iter()
            .all(|icon| node.label() != Some(icon) && node.value() != Some(icon))
    }));
}
