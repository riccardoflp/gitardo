//! The two versions of an image side by side in the Diff page, drawn from the store, on repositories made for each
//! test: their sizes over them, one scale for both, **Empty** for an added picture, the same pixels said, **Image** and
//! **Text** of an SVG, **Stage file**, a binary file that is not an image, `F7` over an image, and the page drawn in
//! each theme.

use app::store::{Message, State};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use testkit::png;
use ui::{Modifiers, NotRead};
use views::Intent;
use views::diff::{DiffFile, DiffImages, DiffTarget, DiffVersion, FileDiff, PageLine};
use views::feature::FeatureIntent;
use views::text::diff as words;

use super::diff::{DIFF, f7, head_to_working_tree, open, press, shown};
use super::image::{commit_bytes, png_of};
use super::showing;
use super::support::{OperationsDriver, Window, has, says};

/// The images of the first file of the page shown.
fn images(state: &State) -> &DiffImages {
    match shown(state).diffs.first() {
        Some(FileDiff::Image(images)) => images,
        other => panic!("the file shows its pictures, not {other:?}"),
    }
}

/// The width and height of a side read.
fn size_of(side: Option<&Result<ui::ImageRead, NotRead>>) -> (u32, u32) {
    let read = side.unwrap().as_ref().unwrap();
    (read.width, read.height)
}

/// A repository with `logo.png` of 100 × 100 committed and made 200 × 200 in the working tree.
fn grown() -> Repository {
    let repository = Repository::new();
    let _ = commit_bytes(
        &repository,
        "logo.png",
        &png_of(100, 100, [200, 40, 40, 255]),
        "Add the logo",
    );
    std::fs::write(repository.path().join("logo.png"), png_of(200, 200, [40, 40, 200, 255])).unwrap();
    repository
}

#[test]
fn diff_image__a_png_changed_in_the_working_tree_shows_head_and_the_working_tree_drawn_with_their_sizes_over_them() {
    let repository = grown();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("logo.png"));
    let images = images(&state);
    assert_eq!(size_of(images.left.as_ref()), (100, 100));
    assert_eq!(size_of(images.right.as_ref()), (200, 200));
    let (_, tree) = Window::new(ThemeName::Dark, DIFF).draw(&state, &[]);
    assert!(says(&tree, "HEAD") && says(&tree, "Working tree"));
    let said = views::diff::what_changed(images);
    assert!(
        said.starts_with("100 \u{d7} 100 \u{2192} 200 \u{d7} 200 \u{b7} "),
        "{said}"
    );
    assert!(says(&tree, &said));
}

#[test]
fn diff_image__a_png_of_100_that_became_200_is_drawn_half_as_wide_on_the_left_as_on_the_right() {
    let repository = grown();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("logo.png"));
    let mut window = Window::new(ThemeName::Dark, DIFF);
    let _ = window.draw(&state, &[]);
    let width_of = |color: render::Color| {
        let columns: Vec<i32> = (0..1000)
            .filter(|&column| (0..600).any(|row| window.frame.pixel(column, row) == Some(color)))
            .collect();
        columns.len()
    };
    let (left, right) = (
        width_of(render::Color::rgb(200, 40, 40)),
        width_of(render::Color::rgb(40, 40, 200)),
    );
    assert!(right > 0 && (left * 2).abs_diff(right) <= 2, "{left} and {right}");
}

#[test]
fn diff_image__an_added_png_shows_empty_on_the_left_and_the_picture_on_the_right() {
    let mut repository = Repository::new();
    let _ = repository.commit("Start", &[("README.md", "# Start\n")]);
    std::fs::write(repository.path().join("new.png"), png_of(30, 20, [0, 120, 0, 255])).unwrap();
    let mut state = showing(repository.path().to_path_buf());
    let target = DiffTarget::file(DiffVersion::Empty, DiffVersion::WorkingTree, DiffFile::same("new.png"));
    open(&mut state, target);
    let images = images(&state);
    assert!(images.left.is_none());
    assert_eq!(size_of(images.right.as_ref()), (30, 20));
    let (_, tree) = Window::new(ThemeName::Dark, DIFF).draw(&state, &[]);
    assert!(says(&tree, words::EMPTY));
}

#[test]
fn diff_image__a_png_saved_again_with_the_same_pixels_says_only_the_bytes_changed() {
    let repository = Repository::new();
    let picture = png::Image::filled(10, 10, [9, 9, 9, 255]);
    let _ = commit_bytes(&repository, "same.png", &png::encode(&picture).unwrap(), "Add it");
    let mut bytes = png::encode(&picture).unwrap();
    // A private chunk the decoder passes over, before the end: the same pixels in other bytes.
    let end = bytes.len() - 12;
    let mut chunk = vec![0, 0, 0, 1];
    chunk.extend_from_slice(b"prVta");
    chunk.extend_from_slice(&crc32(b"prVta").to_be_bytes());
    let _ = bytes.splice(end..end, chunk);
    std::fs::write(repository.path().join("same.png"), &bytes).unwrap();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("same.png"));
    assert!(images(&state).is_same_pixels());
    let (_, tree) = Window::new(ThemeName::Dark, DIFF).draw(&state, &[]);
    assert!(says(&tree, words::SAME_PIXELS));
}

/// The CRC-32 of PNG, over the type and the data of a chunk.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[test]
fn diff_image__an_svg_opens_on_image_and_text_shows_the_diff_of_its_lines_editable_on_the_right() {
    let repository = Repository::new();
    let before = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#ff0000"/></svg>
"##;
    let _ = commit_bytes(&repository, "check.svg", before.as_bytes(), "Add the check");
    std::fs::write(
        repository.path().join("check.svg"),
        before.replace("#ff0000", "#00ff00"),
    )
    .unwrap();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("check.svg"));
    assert!(images(&state).text.is_some());
    let mut window = Window::new(ThemeName::Dark, DIFF);
    let intents = window.click(&state, Role::Button, words::TEXT);
    for intent in intents {
        let _ = super::support::run(&mut state, Message::Intent(intent));
    }
    let Some(FileDiff::Text(text)) = shown(&state).diffs.first() else {
        panic!("Text shows the diff of the lines");
    };
    assert_eq!(text.changes.len(), 1);
    assert!(shown(&state).lines.iter().any(|line| matches!(line, PageLine::Row(..))));
    let intents = window.click(&state, Role::Button, words::IMAGE);
    for intent in intents {
        let _ = super::support::run(&mut state, Message::Intent(intent));
    }
    assert!(images(&state).text.is_some());
}

#[test]
fn diff_image__stage_file_on_an_image_stages_it_and_the_page_against_the_index_has_no_change_left() {
    let repository = grown();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("logo.png"));
    let mut window = Window::new(ThemeName::Dark, DIFF);
    let intents = window.click(&state, Role::Button, words::STAGE_FILE);
    assert!(
        intents
            .iter()
            .any(|intent| matches!(intent, Intent::Feature(FeatureIntent::Run(..))))
    );
    let driver = OperationsDriver::new();
    for intent in intents {
        driver.intent(&mut state, intent);
    }
    loop {
        match driver.next(&mut state) {
            Message::FeatureConfirms(_) => driver.intent(&mut state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureEnded { .. } => break,
            _ => {}
        }
    }
    assert!(
        repository
            .git(&["diff", "--cached", "--name-only"])
            .contains("logo.png")
    );
    let target = DiffTarget::file(DiffVersion::Index, DiffVersion::WorkingTree, DiffFile::same("logo.png"));
    open(&mut state, target);
    assert!(images(&state).is_same_bytes);
    assert_eq!(shown(&state).changes, []);
}

#[test]
fn diff_image__a_file_with_a_nul_byte_that_is_not_an_image_still_refuses_as_binary() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the data", &[("data.bin", "head\0er\n")]);
    std::fs::write(repository.path().join("data.bin"), "head\0er, changed\n").unwrap();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("data.bin"));
    assert_eq!(state.notice, Some(views::Notice::DiffLooksBinary));
}

#[test]
fn diff_image__in_a_page_of_a_text_file_and_an_image_f7_goes_from_the_text_to_the_image_and_on_to_the_next_file() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the files", &[("a.rs", "one\ntwo\n"), ("b.rs", "three\n")]);
    let _ = commit_bytes(&repository, "logo.png", &png_of(10, 10, [1, 1, 1, 255]), "Add the logo");
    std::fs::write(repository.path().join("a.rs"), "one\nchanged\n").unwrap();
    std::fs::write(repository.path().join("b.rs"), "changed\n").unwrap();
    std::fs::write(repository.path().join("logo.png"), png_of(12, 12, [2, 2, 2, 255])).unwrap();
    let mut state = showing(repository.path().to_path_buf());
    let target = DiffTarget {
        left: DiffVersion::Head,
        right: DiffVersion::WorkingTree,
        files: vec![
            DiffFile::same("a.rs"),
            DiffFile::same("logo.png"),
            DiffFile::same("b.rs"),
        ],
        title: Some(String::from("Three files")),
    };
    open(&mut state, target);
    let mut window = Window::new(ThemeName::Dark, DIFF);
    let current_file = |state: &State| {
        let page = shown(state);
        page.current
            .and_then(|current| page.changes.get(current))
            .map(|place| place.file)
    };
    let _ = press(&mut window, &mut state, &[f7(Modifiers::NONE)]);
    assert_eq!(current_file(&state), Some(0));
    let _ = press(&mut window, &mut state, &[f7(Modifiers::NONE)]);
    assert_eq!(current_file(&state), Some(1));
    let _ = press(&mut window, &mut state, &[f7(Modifiers::NONE)]);
    assert_eq!(current_file(&state), Some(2));
}

#[test]
fn the_diff_page_of_an_image_is_drawn_in_each_theme() {
    let repository = Repository::new();
    let mut picture = png::Image::filled(96, 64, [40, 120, 220, 255]);
    for column in 0..48 {
        for row in 0..32 {
            picture.set_pixel(column, row, [0, 0, 0, 0]);
        }
    }
    let _ = commit_bytes(&repository, "logo.png", &png::encode(&picture).unwrap(), "Add the logo");
    std::fs::write(repository.path().join("logo.png"), png_of(48, 48, [220, 120, 40, 255])).unwrap();
    for (theme, name) in [
        (ThemeName::Dark, "diff-image-dark"),
        (ThemeName::Light, "diff-image-light"),
    ] {
        let mut state = showing(repository.path().to_path_buf());
        open(&mut state, head_to_working_tree("logo.png"));
        let mut window = Window::new(theme, (800, 420));
        let (_, tree) = window.draw(&state, &[]);
        assert!(!has(&tree, Role::Button, words::IMAGE));
        window.assert_snapshot(name);
    }
}
