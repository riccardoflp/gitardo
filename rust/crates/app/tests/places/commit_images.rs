//! The images of a commit in its page, drawn from the store, on repositories made for each test: the two versions of
//! a PNG in place of **Binary file.**, **Empty** for a PNG added, nothing read while the diff is folded, a click that
//! opens the Diff page, **Show the images** of an SVG, a comparison of two branches, and the page drawn in each theme.

use app::store::State;
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::commit_page::{CommitPageId, CommitPageIntent, PageImage};
use views::diff::{DiffImages, DiffIntent, DiffVersion, FileDiff};
use views::text::details as words;

use super::commit_page::{PAGE, ask_page, open};
use super::compare::compared;
use super::image::{commit_bytes, png_of};
use super::showing;
use super::support::{Window, has, says};

/// A square SVG of one color.
fn svg_of(color: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12"><rect width="12" height="12" fill="{color}"/></svg>
"#
    )
}

/// A repository whose last commit made `media/logo.png` of 40 × 40 into one of 80 × 60.
fn repository() -> Repository {
    let repository = Repository::new();
    let _ = commit_bytes(
        &repository,
        "media/logo.png",
        &png_of(40, 40, [200, 40, 40, 255]),
        "Add the logo",
    );
    let _ = commit_bytes(
        &repository,
        "media/logo.png",
        &png_of(80, 60, [40, 40, 200, 255]),
        "Make the logo wider",
    );
    repository
}

/// The two versions read of a file of a page, by its place.
fn read(state: &State, id: CommitPageId, file: usize) -> Option<&DiffImages> {
    let page = state.commit_pages.pages.iter().find(|page| page.id == id)?;
    match page.images.files.get(&file) {
        Some(PageImage::Read(Some(images))) => Some(images),
        _ => None,
    }
}

/// The width and height of a side read.
fn size_of(side: Option<&Result<ui::ImageRead, ui::NotRead>>) -> (u32, u32) {
    let read = side.unwrap().as_ref().unwrap();
    (read.width, read.height)
}

/// The window of a store drawn once, and its tree.
fn drawn(state: &State) -> TreeUpdate {
    Window::new(ThemeName::Dark, PAGE).draw(state, &[]).1
}

#[test]
fn commit_page_images__a_commit_that_changed_media_logo_png_shows_its_two_versions_in_place_of_binary_file() {
    let repository = repository();
    let mut state = showing(repository.path().to_path_buf());
    let id = open(&mut state, "Make the logo wider");
    ask_page(&mut state, id, CommitPageIntent::ShowChanges(true));
    let images = read(&state, id, 0).expect("the two versions are read");
    assert_eq!(size_of(images.left.as_ref()), (40, 40));
    assert_eq!(size_of(images.right.as_ref()), (80, 60));
    let tree = drawn(&state);
    assert!(!says(&tree, words::BINARY_FILE));
    assert!(has(&tree, Role::Button, &words::open_images("media/logo.png")));
}

#[test]
fn commit_page_images__a_commit_that_added_a_png_shows_empty_on_the_left_and_the_picture_on_the_right() {
    let mut repository = Repository::new();
    let _ = repository.commit("Start", &[("README.md", "# Start\n")]);
    let _ = commit_bytes(
        &repository,
        "new.png",
        &png_of(30, 20, [0, 120, 0, 255]),
        "Add a picture",
    );
    let mut state = showing(repository.path().to_path_buf());
    let id = open(&mut state, "Add a picture");
    ask_page(&mut state, id, CommitPageIntent::ShowChanges(true));
    let images = read(&state, id, 0).expect("the two versions are read");
    assert!(images.left.is_none());
    assert_eq!(size_of(images.right.as_ref()), (30, 20));
    assert!(says(&drawn(&state), views::text::diff::EMPTY));
}

#[test]
fn commit_page_images__a_folded_diff_of_an_image_reads_nothing_until_it_is_unfolded() {
    let repository = repository();
    let mut state = showing(repository.path().to_path_buf());
    let id = open(&mut state, "Make the logo wider");
    let is_unread = |state: &State| {
        let page = state.commit_pages.pages.iter().find(|page| page.id == id).unwrap();
        page.images.files.is_empty() && page.image_jobs.is_empty()
    };
    assert!(is_unread(&state), "nothing is read when the page opens");
    ask_page(
        &mut state,
        id,
        CommitPageIntent::Fold(0, model::commit_page::Fold::Folded),
    );
    ask_page(&mut state, id, CommitPageIntent::ShowChanges(true));
    assert!(is_unread(&state), "a diff folded reads nothing");
    ask_page(
        &mut state,
        id,
        CommitPageIntent::Fold(0, model::commit_page::Fold::Unfolded),
    );
    assert!(read(&state, id, 0).is_some());
}

#[test]
fn commit_page_images__a_click_on_the_picture_opens_the_diff_page_of_the_file_at_that_commit() {
    let repository = repository();
    let mut state = showing(repository.path().to_path_buf());
    let id = open(&mut state, "Make the logo wider");
    ask_page(&mut state, id, CommitPageIntent::ShowChanges(true));
    let mut window = Window::new(ThemeName::Dark, PAGE);
    let intents = window.click(&state, Role::Button, &words::open_images("media/logo.png"));
    let [Intent::Diff(DiffIntent::Open(target))] = intents.as_slice() else {
        panic!("the Diff page opens, not {intents:?}");
    };
    let commit = model::CommitId::parse(repository.git(&["rev-parse", "HEAD"]).trim()).unwrap();
    assert_eq!(target.right, DiffVersion::commit(commit));
    assert_eq!(
        target.files.first().map(|file| file.right.as_str()),
        Some("media/logo.png")
    );
}

#[test]
fn commit_page_images__an_svg_shows_its_lines_and_show_the_images_shows_its_two_pictures() {
    let repository = Repository::new();
    let _ = commit_bytes(&repository, "check.svg", svg_of("#ff0000").as_bytes(), "Add the check");
    let _ = commit_bytes(
        &repository,
        "check.svg",
        svg_of("#00ff00").as_bytes(),
        "Make the check green",
    );
    let mut state = showing(repository.path().to_path_buf());
    let id = open(&mut state, "Make the check green");
    ask_page(&mut state, id, CommitPageIntent::ShowChanges(true));
    let tree = drawn(&state);
    assert!(has(&tree, Role::Button, words::SHOW_THE_IMAGES));
    assert!(!has(&tree, Role::Button, &words::open_images("check.svg")));
    assert!(
        read(&state, id, 0).is_none(),
        "an SVG shows its lines, and reads no picture"
    );
    let intents = Window::new(ThemeName::Dark, PAGE).click(&state, Role::Button, words::SHOW_THE_IMAGES);
    assert_eq!(intents, [Intent::CommitPage(id, CommitPageIntent::ShowImages(0, true))]);
    ask_page(&mut state, id, CommitPageIntent::ShowImages(0, true));
    let images = read(&state, id, 0).expect("its two pictures are read");
    assert_eq!(images.left.as_ref().unwrap().as_ref().unwrap().width, 12);
    let tree = drawn(&state);
    assert!(has(&tree, Role::Button, words::SHOW_THE_LINES));
    assert!(has(&tree, Role::Button, &words::open_images("check.svg")));
}

#[test]
fn commit_page_images__a_comparison_of_two_branches_that_differ_in_a_png_shows_its_two_versions() {
    let repository = Repository::new();
    let _ = commit_bytes(&repository, "logo.png", &png_of(10, 10, [1, 2, 3, 255]), "Add the logo");
    repository.branch("feature");
    repository.checkout("feature");
    let _ = commit_bytes(
        &repository,
        "logo.png",
        &png_of(20, 10, [3, 2, 1, 255]),
        "Make the logo wider",
    );
    repository.checkout("main");
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let mut window = Window::new(ThemeName::Dark, (1200, 900));
    for intent in window.click(&state, Role::Button, "logo.png") {
        driver.intent(&mut state, intent);
    }
    let page = state.diff_pages.pages.last().unwrap();
    let Some(FileDiff::Image(images)) = page.diffs.first() else {
        panic!("the file shows its two pictures, not {:?}", page.diffs.first());
    };
    assert_eq!(size_of(images.left.as_ref()), (10, 10));
    assert_eq!(size_of(images.right.as_ref()), (20, 10));
}

#[test]
fn the_commit_page_with_an_image_is_drawn_in_each_theme() {
    let repository = repository();
    for (theme, name) in [
        (ThemeName::Dark, "commit-images-dark"),
        (ThemeName::Light, "commit-images-light"),
    ] {
        let mut state = showing(repository.path().to_path_buf());
        let id = open(&mut state, "Make the logo wider");
        ask_page(&mut state, id, CommitPageIntent::ShowFile(0));
        let mut window = Window::new(theme, (900, 520));
        let _ = window.draw(&state, &[]);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(name);
    }
}
