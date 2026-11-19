//! An image file in the File page, drawn from the store, on repositories made for each test: its picture fit to the
//! page with the line under it, zoomed with the keys, **Preview** and **Source** of an SVG, a picture drawn again when
//! its file changes, an image that cannot be read, an image at a revision, and the page drawn in each theme.

use std::time::Duration;

use app::store::{Message, State};
use model::repo::Change;
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use testkit::png;
use ui::editor::Command;
use ui::{Modifiers, Rect};
use views::file_view::{FilePageIntent, FileTarget, ImageIntent, ImageShown, fit_zoom};
use views::text::file_view as words;

use super::file_view::{FILE, open, shown, step};
use super::markdown::{ask_page, click, drawn};
use super::showing;
use super::support::{self, Window, has, says, says_in, shortcut};

/// A square SVG of one color.
const CHECK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><rect width="24" height="24" fill="#20a040"/></svg>"##;

/// Writes a file of bytes into the working tree and commits it; gives the full id of the commit.
pub(super) fn commit_bytes(repository: &Repository, path: &str, bytes: &[u8], message: &str) -> String {
    let file = repository.path().join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, bytes).unwrap();
    let _ = repository.git(&["add", "--", path]);
    let _ = repository.git(&["commit", "--quiet", "-m", message]);
    repository.git(&["rev-parse", "HEAD"]).trim_end().to_owned()
}

/// The bytes of a PNG of one color.
pub(super) fn png_of(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
    png::encode(&png::Image::filled(width, height, color)).unwrap()
}

/// A repository with `media/logo.png`, a picture of 512 × 512, and `icons/check.svg`.
fn repository() -> Repository {
    let repository = Repository::new();
    let _ = commit_bytes(
        &repository,
        "media/logo.png",
        &png_of(512, 512, [200, 40, 40, 255]),
        "Add the logo",
    );
    let _ = commit_bytes(&repository, "icons/check.svg", CHECK.as_bytes(), "Add the check");
    repository
}

/// What the page shown draws of its picture.
fn picture(state: &State) -> ImageShown<'_> {
    shown(state).image.as_ref().unwrap().shown()
}

/// The repository, a page of one of its files, and a window that drew it once.
fn page_of(repository: &Repository, path: &str) -> (State, Window) {
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, FileTarget::working_tree(path));
    let mut window = Window::new(ThemeName::Dark, FILE);
    let _ = step(&mut window, &mut state, &[]);
    (state, window)
}

#[test]
fn file_view_image__logo_png_opens_a_page_with_its_picture_and_its_size_format_and_weight_under_it() {
    let repository = repository();
    let (state, mut window) = page_of(&repository, "media/logo.png");
    assert_eq!(shown(&state).title, "logo.png");
    let ImageShown::Drawn(read) = picture(&state) else {
        panic!("the picture is drawn");
    };
    assert_eq!((read.width, read.height), (512, 512));
    let weight = std::fs::metadata(repository.path().join("media/logo.png"))
        .unwrap()
        .len();
    let tree = drawn(&mut window, &state);
    let line = format!(
        "512 \u{d7} 512 \u{b7} PNG \u{b7} {}",
        views::text::worktree_setup::bytes(weight)
    );
    assert!(says(&tree, &line), "{line}");
}

#[test]
fn file_view_image__a_picture_of_4000_by_3000_in_a_page_800_wide_is_drawn_800_wide_and_one_of_16_by_16_16_wide() {
    let page = Rect::new(0.0, 0.0, 800.0, 700.0);
    let large = fit_zoom((4000.0, 3000.0), page);
    assert!((4000.0 * large - 800.0).abs() < 0.01);
    assert!((fit_zoom((16.0, 16.0), page) - 1.0).abs() < f32::EPSILON);
}

#[test]
fn file_view_image__ctrl_equals_twice_from_100_percent_draws_it_at_400_percent_its_pixels_square_and_ctrl_0_fits() {
    let repository = Repository::new();
    let mut checker = png::Image::filled(2, 2, [255, 0, 0, 255]);
    checker.set_pixel(0, 0, [0, 0, 255, 255]);
    checker.set_pixel(1, 1, [0, 0, 255, 255]);
    let _ = commit_bytes(
        &repository,
        "small.png",
        &png::encode(&checker).unwrap(),
        "Add a small picture",
    );
    let (mut state, mut window) = page_of(&repository, "small.png");
    let zoom_in = shortcut('=', Modifiers::CONTROL);
    let _ = step(&mut window, &mut state, std::slice::from_ref(&zoom_in));
    let _ = step(&mut window, &mut state, &[zoom_in]);
    assert_eq!(shown(&state).image.as_ref().unwrap().place.zoom, Some(4.0));
    let tree = step(&mut window, &mut state, &[]);
    assert!(says_in(&tree, "\u{b7} 400%"));
    let _ = window.draw(&state, &[]);
    let (red, blue) = (render::Color::rgb(255, 0, 0), render::Color::rgb(0, 0, 255));
    let inked: Vec<(i32, i32)> = (0..to_coordinate(FILE.1))
        .flat_map(|row| (0..to_coordinate(FILE.0)).map(move |column| (column, row)))
        .filter(
            |&(column, row)| matches!(window.frame.pixel(column, row), Some(color) if color == red || color == blue),
        )
        .collect();
    let left = inked.iter().map(|point| point.0).min().unwrap();
    let right = inked.iter().map(|point| point.0).max().unwrap();
    let top = inked.iter().map(|point| point.1).min().unwrap();
    let bottom = inked.iter().map(|point| point.1).max().unwrap();
    assert_eq!((right - left + 1, bottom - top + 1), (8, 8));
    assert_eq!(inked.len(), 64);
    assert_eq!(window.frame.pixel(left, top), Some(blue));
    assert_eq!(window.frame.pixel(left + 4, top), Some(red));
    let _ = step(&mut window, &mut state, &[shortcut('0', Modifiers::CONTROL)]);
    assert_eq!(shown(&state).image.as_ref().unwrap().place.zoom, None);
}

#[test]
fn file_view_image__check_svg_opens_in_preview_source_shows_its_text_and_an_edit_of_its_fill_shows_in_preview() {
    let repository = repository();
    let (mut state, mut window) = page_of(&repository, "icons/check.svg");
    let ImageShown::Drawn(read) = picture(&state) else {
        panic!("the SVG is drawn");
    };
    assert_eq!(read.image.width(), 48);
    let first = read.image.key();
    let _ = click(&mut window, &mut state, (Role::Button, words::SOURCE));
    assert!(!shown(&state).image.as_ref().unwrap().is_preview);
    let _ = step(&mut window, &mut state, &[]);
    let text = super::file_view::text(&state);
    assert!(text.contains("fill=\"#20a040\""));
    let edited = text.replace("#20a040", "#0000ff");
    let _ = ask_page(
        &mut state,
        FilePageIntent::Edit(vec![Command::SelectAll, Command::Paste(edited)]),
    );
    let _ = ask_page(&mut state, FilePageIntent::Image(ImageIntent::ShowPreview));
    let ImageShown::Drawn(read) = picture(&state) else {
        panic!("the edited SVG is drawn");
    };
    assert_ne!(read.image.key(), first);
}

#[test]
fn file_view_image__saving_the_png_from_another_program_draws_the_new_picture() {
    let repository = repository();
    let (mut state, _window) = page_of(&repository, "media/logo.png");
    std::fs::write(
        repository.path().join("media/logo.png"),
        png_of(64, 32, [0, 0, 255, 255]),
    )
    .unwrap();
    let changes = vec![Change::WorkingTree("media/logo.png".into())];
    let _ = support::run(
        &mut state,
        Message::Changed {
            changes,
            at: Duration::from_secs(10),
        },
    );
    let _ = support::run(
        &mut state,
        Message::Wake {
            at: Duration::from_secs(20),
        },
    );
    let ImageShown::Drawn(read) = picture(&state) else {
        panic!("the new picture is drawn");
    };
    assert_eq!((read.width, read.height), (64, 32));
}

#[test]
fn file_view_image__a_png_cut_short_says_this_image_could_not_be_read_and_nothing_else() {
    let repository = Repository::new();
    let bytes = png_of(32, 32, [1, 2, 3, 255]);
    let _ = commit_bytes(&repository, "cut.png", &bytes[..40], "Add a broken picture");
    let (state, mut window) = page_of(&repository, "cut.png");
    let tree = drawn(&mut window, &state);
    assert!(says(&tree, words::IMAGE_UNREADABLE));
    assert!(!says(&tree, words::NOT_TEXT));
    assert!(!says_in(&tree, "PNG \u{b7}"));
}

#[test]
fn file_view_image__logo_png_at_a_commit_opens_a_read_only_page_titled_with_its_hash_with_the_picture_of_that_commit() {
    let repository = repository();
    let first = repository.git(&["rev-parse", "HEAD~1"]).trim_end().to_owned();
    let _ = commit_bytes(
        &repository,
        "media/logo.png",
        &png_of(8, 8, [0, 0, 0, 255]),
        "Make the logo smaller",
    );
    let mut state = showing(repository.path().to_path_buf());
    let commit = model::CommitId::parse(&first).unwrap();
    let target = FileTarget {
        path: String::from("media/logo.png"),
        revision: Some(commit.clone()),
        line: None,
    };
    open(&mut state, target);
    assert_eq!(shown(&state).title, format!("logo.png ({})", commit.short()));
    let ImageShown::Drawn(read) = picture(&state) else {
        panic!("the picture of that commit is drawn");
    };
    assert_eq!((read.width, read.height), (512, 512));
    let mut window = Window::new(ThemeName::Dark, FILE);
    let tree = drawn(&mut window, &state);
    assert!(!has(&tree, Role::Button, words::OPEN_EXTERNAL));
}

#[test]
fn the_file_page_of_an_image_is_drawn_in_each_theme() {
    let repository = Repository::new();
    let mut picture = png::Image::filled(96, 64, [40, 120, 220, 255]);
    for column in 0..48 {
        for row in 0..32 {
            picture.set_pixel(column, row, [0, 0, 0, 0]);
        }
    }
    let _ = commit_bytes(
        &repository,
        "media/logo.png",
        &png::encode(&picture).unwrap(),
        "Add the logo",
    );
    for (theme, name) in [
        (ThemeName::Dark, "file-image-dark"),
        (ThemeName::Light, "file-image-light"),
    ] {
        let mut state = showing(repository.path().to_path_buf());
        open(&mut state, FileTarget::working_tree("media/logo.png"));
        let mut window = Window::new(theme, (700, 400));
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(name);
    }
}

/// A pixel of the window as a coordinate of its frame.
fn to_coordinate(value: u32) -> i32 {
    i32::try_from(value).unwrap()
}
