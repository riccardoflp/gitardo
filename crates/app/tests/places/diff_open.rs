//! The File page opened from a Diff page, on repositories made for each test: **Open file** (`diff.open-file`), and
//! the line of a side opened with `F4` or **Jump to source** (`diff.open-at-line`).

use platform::accesskit::Role;
use render::ThemeName;

use super::diff::{DIFF, head_to_working_tree, open, repository_with_three_changes};
use super::file_view::shown;
use super::pointer::apply;
use super::showing;
use super::support::Window;

#[test]
fn diff_open_file__open_file_on_the_diff_of_app_rs_opens_its_file_page_from_the_working_tree() {
    let repository = repository_with_three_changes();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("app.rs"));
    let mut window = Window::new(ThemeName::Dark, DIFF);
    let intents = window.click(&state, Role::Button, views::text::diff::OPEN_FILE);
    apply(&mut state, intents);
    let page = shown(&state);
    assert_eq!(page.path, "app.rs");
    assert_eq!(page.revision, None, "the file as it is on disk");
}
