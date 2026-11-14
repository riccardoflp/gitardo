//! The page of a commit drawn from the store, its commit and its lines read by the workers from repositories made for
//! each test: how it opens, what it shows, its diffs shown and folded, and find in page over them.

use std::path::PathBuf;

use app::persisted::Persisted;
use app::store::{Message, PageReadState, State};
use model::CommitId;
use model::commit_page::{Fold, PatchLines};
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use views::Content;
use views::Intent;
use views::commit_page::{CommitPageId, CommitPageIntent, CommitPageState, PageTarget};
use views::details::{DetailsIntent, FileLayout, FilesIntent};
use views::graph::GraphIntent;
use views::page::{Page, PageState};

use super::inspector::{ask, row_id};
use super::support::{self, NOW, Window, find, has, says, says_in, shortcut, typed};
use super::{repository_with_history, showing};

/// The size of the window of the pages.
pub const PAGE: (u32, u32) = (1000, 860);

/// A message whose text under its title is three paragraphs.
const MESSAGE: &str = "Explain the timeline in the guide\n\n\
    The guide starts with the timeline, where each row is a\n\
    commit and the labels name the branches, the tags and\n\
    the remote branches of the repository.\n\n\
    The labels name the branches.\n\n\
    Signed-off-by: Test Author <author@example.com>";

/// A file of so many lines, each its number.
pub fn lines(count: usize, word: &str) -> String {
    let mut text = String::new();
    for line in 0..count {
        text.push_str(word);
        text.push(' ');
        text.push_str(&line.to_string());
        text.push('\n');
    }
    text
}

/// A repository whose last commit has a message of several paragraphs and changes a short file, a long file and one
/// with a word to find, after a merge of `feature`.
pub fn repository_for_page() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    repository.branch("feature");
    repository.checkout("feature");
    let _ = repository.commit(
        "Add the parser",
        &[("src/parse/lexer.rs", "lex\n"), ("src/parse/tree.rs", "tree\n")],
    );
    repository.checkout("main");
    let _ = repository.commit("Change the readme", &[("README.md", "Gitardo, the timeline\n")]);
    repository.merge("feature");
    let long = lines(900, "long");
    let _ = repository.commit(
        MESSAGE,
        &[
            ("docs/guide.md", "The timeline\nA haystack with a needle in it\n"),
            ("src/app/main.rs", "fn main() {}\n"),
            ("data/long.txt", &long),
        ],
    );
    repository
}

/// Opens the page of the row of the timeline whose subject starts so, as a double click does, and reads it.
pub fn open(state: &mut State, subject: &str) -> CommitPageId {
    let commit = row_id(state, subject);
    ask(state, Intent::Graph(GraphIntent::Open(commit)));
    shown_page(state)
}

/// The page shown.
pub fn shown_page(state: &State) -> CommitPageId {
    match state.page_shown() {
        Some(Page::Commit(id)) => id,
        other => panic!("a page of a commit is shown, not {other:?}"),
    }
}

/// The page of a commit as the store keeps it.
fn page(state: &State, id: CommitPageId) -> &app::store::CommitPage {
    state.commit_pages.pages.iter().find(|page| page.id == id).unwrap()
}

/// What the page shown is drawn from.
fn page_state(state: &State, check: impl FnOnce(&CommitPageState<'_>)) {
    let data = app::store::ViewData::new(state, Vec::new(), NOW);
    let window = data.window_state(state);
    let Content::Page(host) = window.content else {
        panic!("a page is shown");
    };
    let PageState::Commit(page) = host.page else {
        panic!("the page of a commit is shown");
    };
    check(&page);
}

/// Asks something of a page and runs what it starts.
pub fn ask_page(state: &mut State, id: CommitPageId, intent: CommitPageIntent) {
    ask(state, Intent::CommitPage(id, intent));
}

/// The window of a store drawn once, and its tree.
fn drawn(state: &State) -> TreeUpdate {
    Window::new(ThemeName::Dark, PAGE).draw(state, &[]).1
}

#[test]
fn commit_page_open__double_clicking_a_commit_opens_a_page_titled_with_its_short_hash_and_subject() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let commit = row_id(&state, "Show the branch rail");
    let intents = double_click_row(&state, 1);
    assert!(
        intents.contains(&Intent::Graph(GraphIntent::Open(commit.clone()))),
        "{intents:?}"
    );
    ask(&mut state, Intent::Graph(GraphIntent::Open(commit.clone())));
    let id = shown_page(&state);
    let title = format!("{} Show the branch rail", commit.short());
    assert_eq!(page(&state, id).title, title);
    assert!(says(&drawn(&state), &title), "the trail names the page");
}

#[test]
fn welcome_page__opening_a_commit_with_a_double_click_in_the_timeline_ticks_its_step() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    assert!(!state.persisted.welcome_steps[2]);
    for intent in double_click_row(&state, 1) {
        ask(&mut state, intent);
    }
    let _id = shown_page(&state);
    assert!(
        state.persisted.welcome_steps[2],
        "Open a commit, compare two branches is ticked"
    );
}

/// A double click on a row of the timeline, by its place from the top, and the intents it gives.
fn double_click_row(state: &State, row: usize) -> Vec<Intent> {
    let mut window = Window::new(ThemeName::Dark, PAGE);
    let (_, tree) = window.draw(state, &[]);
    let mut rows: Vec<platform::accesskit::Rect> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::ListBoxOption)
        .filter_map(|(_, node)| node.bounds())
        .collect();
    rows.sort_by(|one, other| one.y0.total_cmp(&other.y0));
    let bounds = *rows.get(row).unwrap();
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a window is far smaller than an f32 counts"
    )]
    let point = render::Point::new(
        f64::midpoint(bounds.x0, bounds.x1) as f32,
        f64::midpoint(bounds.y0, bounds.y1) as f32,
    );
    window.click_at(state, point, 2)
}

#[test]
fn graph_view__a_double_click_on_a_commit_opens_its_page() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let commit = row_id(&state, "Read the settings");
    let intents = double_click_row(&state, 2);
    let open = Intent::Graph(GraphIntent::Open(commit.clone()));
    assert!(intents.contains(&open), "{intents:?}");
    ask(&mut state, open);
    assert_eq!(page(&state, shown_page(&state)).commit, commit);
}

#[test]
fn commit_page_open__opening_the_same_commit_again_shows_the_page_it_has() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let first = open(&mut state, "Show the branch rail");
    ask(&mut state, Intent::ShowTimeline);
    let _ = open(&mut state, "Add the readme");
    let commit = row_id(&state, "Show the branch rail");
    ask(&mut state, Intent::Details(DetailsIntent::OpenPage(commit)));
    assert_eq!(shown_page(&state), first);
    assert_eq!(state.commit_pages.pages.len(), 2);
    assert_eq!(state.pages.open.len(), 2);
}

#[test]
fn commit_page_open__the_same_hash_in_two_repositories_of_the_workspace_opens_two_pages() {
    let (repository, _remote) = repository_with_history();
    let copy = Repository::new();
    let other = copy.path().join("copy");
    let _ = repository.git(&["clone", "--quiet", ".", other.to_str().unwrap()]);
    let mut state = super::window();
    let folders = vec![repository.path().to_path_buf(), other.clone()];
    let _ = support::run(&mut state, Message::Started { folders });
    let shown = state.shown_folder().cloned().unwrap();
    let commit = row_id(&state, "Read the settings");
    ask(&mut state, Intent::Graph(GraphIntent::Open(commit.clone())));
    let next = if app::persisted::same_folder(&shown, &other) {
        repository.path().to_path_buf()
    } else {
        other
    };
    ask(&mut state, Intent::SelectRepository(next));
    assert_eq!(
        row_id(&state, "Read the settings"),
        commit,
        "the same hash in the clone"
    );
    ask(&mut state, Intent::Graph(GraphIntent::Open(commit)));
    assert_eq!(state.commit_pages.pages.len(), 2);
    let folders: Vec<&PathBuf> = state.commit_pages.pages.iter().map(|page| &page.folder).collect();
    assert!(!app::persisted::same_folder(folders[0], folders[1]));
}

#[test]
fn commit_page_open__a_subject_longer_than_32_characters_is_cut_with_an_ellipsis_in_the_title() {
    let mut repository = Repository::new();
    let commit = repository.commit(
        "Read the settings of the window at startup",
        &[("README.md", "Gitardo\n")],
    );
    let mut state = showing(repository.path().to_path_buf());
    let id = open(&mut state, "Read the settings");
    let short = &commit[..7];
    assert_eq!(
        page(&state, id).title,
        format!("{short} Read the settings of the window\u{2026}")
    );
}

#[test]
fn commit_page_view__the_page_shows_the_message_in_full_unwrapped_with_no_show_more() {
    let repository = repository_for_page();
    let mut state = showing(repository.path().to_path_buf());
    let _ = open(&mut state, "Explain the timeline");
    let tree = drawn(&state);
    assert!(says(&tree, "Explain the timeline in the guide"));
    let unwrapped = "The guide starts with the timeline, where each row is a commit and the labels name the branches, the \
                     tags and the remote branches of the repository.";
    assert!(says_in(&tree, unwrapped), "unwrapped");
    assert!(says_in(&tree, "Signed-off-by: Test Author"), "the whole message");
    assert!(!has(&tree, Role::Button, "Show more"));
}

#[test]
fn commit_page_view__a_click_on_the_hash_copies_it_and_a_click_on_a_parent_opens_its_page() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let commit = row_id(&state, "Show the branch rail");
    let parent = row_id(&state, "Read the settings");
    let id = open(&mut state, "Show the branch rail");
    let mut window = Window::new(ThemeName::Dark, PAGE);
    let _ = window.click(&state, Role::Button, commit.as_str());
    assert_eq!(window.clipboard.as_deref(), Some(commit.as_str()));
    let intents = window.click(&state, Role::Button, parent.short());
    assert_eq!(
        intents,
        [Intent::CommitPage(id, CommitPageIntent::OpenParent(parent.clone()))]
    );
    ask(&mut state, intents[0].clone());
    let opened = shown_page(&state);
    assert_ne!(opened, id);
    assert_eq!(page(&state, opened).commit, parent);
}

#[test]
fn commit_page_view__the_labels_are_those_of_the_commit_in_the_timeline_the_branch_checked_out_filled() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let commit = row_id(&state, "Show the branch rail");
    let expected = state.timeline.layout.labels.get(&commit).cloned().unwrap();
    let _ = open(&mut state, "Show the branch rail");
    page_state(&state, |page| {
        assert_eq!(page.labels, expected.as_slice());
        let main = page.labels.iter().find(|label| label.name == "main").unwrap();
        assert!(main.is_checked_out, "the branch checked out is filled");
    });
    let other = row_id(&state, "Read the settings");
    let expected = state.timeline.layout.labels.get(&other).cloned().unwrap();
    ask(&mut state, Intent::Details(DetailsIntent::OpenPage(other)));
    page_state(&state, |page| {
        assert_eq!(page.labels, expected.as_slice());
        assert!(page.labels.iter().all(|label| !label.is_checked_out));
    });
}

#[test]
fn commit_page_view__show_the_changes_puts_a_diff_under_the_list_for_every_file_and_hide_removes_them() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let id = open(&mut state, "Show the branch rail");
    let tree = drawn(&state);
    assert!(
        !has(&tree, Role::Button, "Fold"),
        "the changes are not in the page when it opens"
    );
    let intents = Window::new(ThemeName::Dark, PAGE).click(&state, Role::Button, "Show the changes");
    assert_eq!(intents, [Intent::CommitPage(id, CommitPageIntent::ShowChanges(true))]);
    ask(&mut state, intents[0].clone());
    let tree = drawn(&state);
    let diffs = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some("Fold"))
        .count();
    assert_eq!(diffs, 1, "a diff for the one file");
    assert!(says(&tree, "branches"), "its line");
    ask_page(&mut state, id, CommitPageIntent::ShowChanges(false));
    assert!(!has(&drawn(&state), Role::Button, "Fold"));
}

#[test]
fn commit_page_view__the_eye_on_a_file_shows_the_changes_and_scrolls_to_its_diff_unfolded() {
    let repository = repository_for_page();
    let mut state = showing(repository.path().to_path_buf());
    let id = open(&mut state, "Explain the timeline");
    let long = long_file(&state, id);
    assert!(page(&state, id).folds.is_folded(long), "the long diff starts folded");
    ask_page(&mut state, id, CommitPageIntent::ShowFile(long));
    let shown = page(&state, id);
    assert!(shown.shows_changes);
    assert!(!shown.folds.is_folded(long));
    assert!(matches!(shown.reveal, Some((_, PageTarget::Diff(file))) if file == long));
    let mut window = Window::new(ThemeName::Dark, PAGE);
    let _ = window.draw(&state, &[]);
    window.now += std::time::Duration::from_secs(2);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, "long 0"), "the first line of the long diff is in sight");
}

/// The place of the long file among the files of the page.
fn long_file(state: &State, id: CommitPageId) -> usize {
    let PageReadState::Read(data) = &page(state, id).read else {
        panic!("the page is read");
    };
    data.detail
        .files
        .iter()
        .position(|file| file.path == "data/long.txt")
        .unwrap()
}

#[test]
fn commit_page_view__a_diff_folded_at_first_unfolds_with_its_arrow_and_unfold_all_unfolds_every_diff() {
    let repository = repository_for_page();
    let mut state = showing(repository.path().to_path_buf());
    let id = open(&mut state, "Explain the timeline");
    let long = long_file(&state, id);
    ask_page(&mut state, id, CommitPageIntent::ShowChanges(true));
    let tree = drawn(&state);
    let _arrow = find(&tree, Role::Button, "Unfold");
    let intents = Window::new(ThemeName::Dark, PAGE).click(&state, Role::Button, "Unfold");
    assert_eq!(
        intents,
        [Intent::CommitPage(id, CommitPageIntent::Fold(long, Fold::Unfolded))]
    );
    ask(&mut state, intents[0].clone());
    assert!(!page(&state, id).folds.is_folded(long));
    ask_page(&mut state, id, CommitPageIntent::FoldAll(Fold::Folded));
    assert!((0..3).all(|file| page(&state, id).folds.is_folded(file)));
    let intents = Window::new(ThemeName::Dark, PAGE).click(&state, Role::Button, "Unfold all");
    assert_eq!(
        intents,
        [Intent::CommitPage(id, CommitPageIntent::FoldAll(Fold::Unfolded))]
    );
    ask(&mut state, intents[0].clone());
    assert!((0..3).all(|file| !page(&state, id).folds.is_folded(file)));
}

#[test]
fn commit_page_view__a_merge_shows_its_changes_against_its_first_parent_and_says_so_after_the_count() {
    let repository = repository_for_page();
    let mut state = showing(repository.path().to_path_buf());
    let id = open(&mut state, "Merge branch");
    let PageReadState::Read(data) = &page(&state, id).read else {
        panic!("the page is read");
    };
    let paths: Vec<&str> = data.detail.files.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, ["src/parse/lexer.rs", "src/parse/tree.rs"]);
    assert!(says(
        &drawn(&state),
        "2 changed files  +2 \u{2212}0 \u{b7} against the first parent"
    ));
}

#[test]
fn commit_page_view__a_file_past_the_cap_says_it_is_too_long_with_a_link_to_the_diff_page() {
    let mut repository = Repository::new();
    let (first, second, third) = (lines(15_000, "first"), lines(8_000, "second"), lines(3_000, "third"));
    let _ = repository.commit(
        "Add three long files",
        &[("a.txt", &first), ("b.txt", &second), ("c.txt", &third)],
    );
    let mut state = showing(repository.path().to_path_buf());
    let id = open(&mut state, "Add three long files");
    ask_page(&mut state, id, CommitPageIntent::ShowFile(1));
    let mut window = Window::new(ThemeName::Dark, PAGE);
    let _ = window.draw(&state, &[]);
    window.now += std::time::Duration::from_secs(2);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, "Too long to show with the others."));
    let _link = find(&tree, Role::Button, "Open it in the Diff page");
}

#[test]
fn commit_page_view__lines_that_cannot_be_matched_to_the_files_say_so_with_a_link() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let id = open(&mut state, "Show the branch rail");
    for page in &mut state.commit_pages.pages {
        if let PageReadState::Read(data) = &mut page.read {
            data.patch = PatchLines::Unmatched;
        }
    }
    ask_page(&mut state, id, CommitPageIntent::ShowChanges(true));
    let tree = drawn(&state);
    assert!(says(
        &tree,
        "Git lists these changes in a way that cannot be shown here."
    ));
    let _link = find(&tree, Role::Button, "Open in the Diff page");
}

#[test]
fn commit_page_view__a_commit_no_longer_in_the_repository_says_so() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let gone = CommitId::parse("0123456789abcdef0123456789abcdef01234567").unwrap();
    ask(&mut state, Intent::Details(DetailsIntent::OpenPage(gone)));
    let id = shown_page(&state);
    assert_eq!(page(&state, id).read, PageReadState::Missing);
    assert!(says(&drawn(&state), "This commit is not in the repository any more."));
}

#[test]
fn commit_page_view__ctrl_f_finds_text_in_the_diffs_that_are_unfolded() {
    let repository = repository_for_page();
    let mut state = showing(repository.path().to_path_buf());
    let id = open(&mut state, "Explain the timeline");
    let mut window = Window::new(ThemeName::Dark, PAGE);
    let count = |window: &mut Window, state: &State, query: &str| {
        let _ = window.draw(state, &[shortcut('f', ui::Modifiers::CONTROL)]);
        let _ = window.draw(state, &[typed(query)]);
        let (_, tree) = window.draw(state, &[]);
        tree
    };
    let tree = count(&mut window, &state, "needle");
    assert!(says(&tree, "No results"), "the changes are not shown yet");
    ask_page(&mut state, id, CommitPageIntent::ShowChanges(true));
    let mut window = Window::new(ThemeName::Dark, PAGE);
    let tree = count(&mut window, &state, "needle");
    assert!(says(&tree, "1 of 1"), "the line of the guide");
    let long = long_file(&state, id);
    let mut window = Window::new(ThemeName::Dark, PAGE);
    let tree = count(&mut window, &state, "long 899");
    assert!(says(&tree, "No results"), "the long diff is folded");
    ask_page(&mut state, id, CommitPageIntent::Fold(long, Fold::Unfolded));
    let mut window = Window::new(ThemeName::Dark, PAGE);
    let tree = count(&mut window, &state, "long 899");
    assert!(says(&tree, "1 of 1"), "unfolded, out of sight");
}

#[test]
fn commit_page_view__the_layout_chosen_in_one_page_is_the_one_the_next_page_opens_with() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let id = open(&mut state, "Show the branch rail");
    page_state(&state, |page| {
        assert_eq!(page.layout, FileLayout::Tree, "the tree to start with");
    });
    let intents =
        Window::new(ThemeName::Dark, PAGE).click(&state, Role::Button, "One after the other, each with its directory");
    assert_eq!(
        intents,
        [Intent::CommitPage(
            id,
            CommitPageIntent::Files(FilesIntent::Layout(FileLayout::List))
        )]
    );
    ask(&mut state, intents[0].clone());
    let _ = open(&mut state, "Add the readme");
    page_state(&state, |page| assert_eq!(page.layout, FileLayout::List));
    let again = Persisted::from_text(&state.persisted.to_text());
    assert_eq!(again.file_layouts.page, FileLayout::List);
}
