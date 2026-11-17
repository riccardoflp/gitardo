//! The places of the window that open the Diff page: a file of the inspector, **Open all changes in one Diff page**, and
//! the links of the page of a commit.

use std::time::Duration;

use app::store::PageReadState;
use model::commit_page::PatchLines;
use platform::accesskit::{Role, TreeUpdate};
use render::{Point, ThemeName};
use testkit::Repository;
use views::Intent;
use views::commit_page::CommitPageIntent;
use views::diff::{DiffIntent, DiffTarget, DiffVersion};

use super::diff::{shown, text};
use super::inspector::{TALL, ask, repository_for_details, row_id, select};
use super::pointer::{apply, click_at};
use super::showing;
use super::support::{Window, has, says};

/// The middle of the first node that says some words.
pub fn words_at(tree: &TreeUpdate, words: &str) -> Point {
    let found = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.label() == Some(words) || node.value() == Some(words))
        .find_map(|(_, node)| node.bounds());
    let bounds = found.unwrap_or_else(|| panic!("nothing says {words}"));
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a window is far smaller than an f32 counts"
    )]
    Point::new(
        f64::midpoint(bounds.x0, bounds.x1) as f32,
        f64::midpoint(bounds.y0, bounds.y1) as f32,
    )
}

/// The targets of the Diff pages some intents open.
pub fn opened(intents: &[Intent]) -> Vec<DiffTarget> {
    intents
        .iter()
        .filter_map(|intent| match intent {
            Intent::Diff(DiffIntent::Open(target) | DiffIntent::OpenBeside(target))
            | Intent::Changes(views::changes::ChangesIntent::OpenBeside(views::changes::BesideFile::Diff(target))) => {
                Some((**target).clone())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn details_show__a_click_on_a_file_opens_its_diff_page_between_the_first_parent_and_the_commit() {
    let repository = repository_for_details();
    let mut state = showing(repository.path().to_path_buf());
    let commit = select(&mut state, "Explain the timeline");
    let parent = row_id(&state, "Merge branch");
    let mut window = Window::new(ThemeName::Dark, TALL);
    let (_, tree) = window.draw(&state, &[]);
    let intents = click_at(&mut window, &state, words_at(&tree, "guide.md"), 1);
    let targets = opened(&intents);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].left, DiffVersion::commit(parent));
    assert_eq!(targets[0].right, DiffVersion::commit(commit));
    assert_eq!(targets[0].files[0].right, "docs/guide.md");
    apply(&mut state, intents);
    assert_eq!(text(shown(&state)).aligned.changes.len(), 1);
}

#[test]
fn details_show__the_button_next_to_the_count_opens_all_the_files_in_one_page_and_is_not_there_with_one_file() {
    let repository = repository_for_details();
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Explain the timeline");
    let mut window = Window::new(ThemeName::Dark, TALL);
    let intents = window.click(&state, Role::Button, "Open all changes in one Diff page");
    assert_eq!(opened(&intents)[0].files.len(), 3);
    apply(&mut state, intents);
    assert_eq!(shown(&state).diffs.len(), 3);
    assert!(shown(&state).title.contains("Explain the timeline"));
    ask(&mut state, Intent::ShowTimeline);
    let _ = select(&mut state, "Change the readme");
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, "1 changed file"));
    assert!(!has(&tree, Role::Button, "Open all changes in one Diff page"));
}

#[test]
fn commit_page_view__the_link_of_a_file_past_the_cap_opens_that_file_in_the_diff_page() {
    let mut repository = Repository::new();
    let (first, second) = (
        super::commit_page::lines(15_000, "first"),
        super::commit_page::lines(8_000, "second"),
    );
    let _ = repository.commit("Add two long files", &[("a.txt", &first), ("b.txt", &second)]);
    let mut state = showing(repository.path().to_path_buf());
    let id = super::commit_page::open(&mut state, "Add two long files");
    super::commit_page::ask_page(&mut state, id, CommitPageIntent::ShowFile(1));
    let mut window = Window::new(ThemeName::Dark, super::commit_page::PAGE);
    let _ = window.draw(&state, &[]);
    window.now += Duration::from_secs(2);
    let intents = window.click(&state, Role::Button, "Open it in the Diff page");
    let targets = opened(&intents);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].files.len(), 1);
    assert_eq!(targets[0].files[0].right, "b.txt");
    apply(&mut state, intents);
    assert_eq!(text(shown(&state)).aligned.rows.len(), 8_000);
}

#[test]
fn commit_page_view__the_link_under_lines_that_cannot_be_matched_opens_all_the_files_in_the_diff_page() {
    let repository = repository_for_details();
    let mut state = showing(repository.path().to_path_buf());
    let id = super::commit_page::open(&mut state, "Explain the timeline");
    for page in &mut state.commit_pages.pages {
        if let PageReadState::Read(data) = &mut page.read {
            data.patch = PatchLines::Unmatched;
        }
    }
    super::commit_page::ask_page(&mut state, id, CommitPageIntent::ShowChanges(true));
    let mut window = Window::new(ThemeName::Dark, super::commit_page::PAGE);
    let (_, tree) = window.draw(&state, &[]);
    let link = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some("Open in the Diff page"))
        .filter_map(|(_, node)| node.bounds())
        .next_back()
        .unwrap();
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a window is far smaller than an f32 counts"
    )]
    let link = Point::new(
        f64::midpoint(link.x0, link.x1) as f32,
        f64::midpoint(link.y0, link.y1) as f32,
    );
    let targets = opened(&click_at(&mut window, &state, link, 1));
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].files.len(), 3, "every file of the commit, in one page");
}
