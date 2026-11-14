//! The inspector drawn from the store, with its details read by the workers from repositories made for each test:
//! what it shows of a commit, its message cut by **Show more**, and what a click on it asks.

use app::persisted::Persisted;
use app::store::{InspectorRead, Message, State};
use model::CommitId;
use platform::accesskit::{Role, TreeUpdate};
use render::{Rect, ThemeName};
use testkit::Repository;
use views::Intent;
use views::details::{DetailsIntent, FileLayout, FilesIntent, MessageState};
use views::graph::{GraphIntent, Selected};
use views::page::Page;

use super::support::{self, Window, find, has, says, says_in};
use super::{repository_with_history, showing};

/// The size of the window: wide enough for the rail, the timeline and the inspector.
pub const WIDE: (u32, u32) = (1000, 520);
/// The size of a window tall enough for every file under a long message.
pub const TALL: (u32, u32) = (1000, 900);

/// A message whose text under its title is six paragraphs, longer than four lines in any inspector.
const LONG_MESSAGE: &str = "Explain the timeline in the guide\n\n\
    The guide starts with the timeline.\n\n\
    Each row is a commit.\n\n\
    The labels name the branches.\n\n\
    The draft is on top.\n\n\
    A merge folds into one row.\n\n\
    The inspector shows the row selected.";

/// A repository whose last commit has a long message and files in directories, after a merge of `feature`.
pub fn repository_for_details() -> Repository {
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
    let _ = repository.commit(
        LONG_MESSAGE,
        &[
            ("docs/guide.md", "The timeline\n"),
            ("src/app/main.rs", "fn main() {}\n"),
            ("CHANGELOG.md", "Unreleased\n"),
        ],
    );
    repository
}

/// The hash of the row of the timeline whose subject starts so.
pub fn row_id(state: &State, subject: &str) -> CommitId {
    let found = state
        .timeline
        .rows
        .iter()
        .filter_map(model::graph::Row::commit)
        .find(|commit| commit.subject.starts_with(subject));
    let Some(commit) = found else {
        panic!("no row {subject}");
    };
    commit.id.clone()
}

/// Reduces an intent and runs the jobs it starts.
pub fn ask(state: &mut State, intent: Intent) {
    let _effects = support::run(state, Message::Intent(intent));
}

/// Selects the row of the timeline whose subject starts so, and reads its details.
pub fn select(state: &mut State, subject: &str) -> CommitId {
    let commit = row_id(state, subject);
    ask(
        state,
        Intent::Graph(GraphIntent::Select(Selected::Commit(commit.clone()))),
    );
    commit
}

/// The window of a store drawn once, tall enough for the files under a long message, and its tree.
pub fn drawn(state: &State) -> TreeUpdate {
    Window::new(ThemeName::Dark, TALL).draw(state, &[]).1
}

#[test]
fn details_show__selecting_a_commit_shows_its_title_author_date_hash_parents_and_count_of_files() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let commit = select(&mut state, "Show the branch rail");
    let parent = row_id(&state, "Read the settings");
    let tree = drawn(&state);
    for text in [
        "Show the branch rail",
        "Author",
        "Test Author",
        "Date",
        "Commit",
        "Parent",
        "1 changed file",
    ] {
        assert!(says(&tree, text), "the inspector says {text}");
    }
    let _hash = find(&tree, Role::Button, commit.short());
    let _parent = find(&tree, Role::Button, parent.short());
}

#[test]
fn details_show__the_committer_is_shown_only_when_its_name_or_date_differs_from_the_author() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Show the branch rail");
    assert!(!says(&drawn(&state), "Committer"), "the same author and committer");
    let _ = repository.git(&[
        "commit",
        "--amend",
        "--quiet",
        "--no-edit",
        "--date=2024-06-01T10:00:00Z",
    ]);
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Show the branch rail");
    assert!(says(&drawn(&state), "Committer"), "the author's date differs");
}

#[test]
fn details_show__a_click_on_the_hash_puts_the_full_hash_on_the_clipboard() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let commit = select(&mut state, "Show the branch rail");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = window.click(&state, Role::Button, commit.short());
    assert_eq!(window.clipboard.as_deref(), Some(commit.as_str()));
    assert_eq!(commit.as_str().len(), 40);
}

#[test]
fn details_show__a_click_on_a_parent_selects_that_commit_in_the_timeline() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Show the branch rail");
    let parent = row_id(&state, "Read the settings");
    let intents = Window::new(ThemeName::Dark, WIDE).click(&state, Role::Button, parent.short());
    assert_eq!(intents, [Intent::Graph(GraphIntent::GoTo(parent.clone()))]);
    ask(&mut state, intents[0].clone());
    assert_eq!(state.timeline.selected, Some(Selected::Commit(parent)));
    assert!(says(&drawn(&state), "Read the settings at startup"));
}

#[test]
fn details_show__a_merge_says_against_the_first_parent_after_the_count() {
    let repository = repository_for_details();
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Merge branch");
    let tree = drawn(&state);
    assert!(says(&tree, "2 changed files \u{b7} against the first parent"));
    let _ = select(&mut state, "Change the readme");
    assert!(!says_in(&drawn(&state), "against the first parent"));
}

#[test]
fn details_show__the_layout_chosen_is_the_one_the_inspector_has_after_the_window_is_opened_again() {
    let repository = repository_for_details();
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Explain the timeline");
    assert_eq!(
        state.persisted.file_layouts.inspector,
        FileLayout::List,
        "the list to start with"
    );
    let intents = Window::new(ThemeName::Dark, WIDE).click(&state, Role::Button, "Under their directories");
    assert_eq!(
        intents,
        [Intent::Details(DetailsIntent::Files(FilesIntent::Layout(
            FileLayout::Tree
        )))]
    );
    let effects = support::run(&mut state, Message::Intent(intents[0].clone()));
    let written = effects.iter().find_map(|effect| match effect {
        app::store::Effect::WritePersisted(persisted) => Some(persisted.clone()),
        _ => None,
    });
    let again = Persisted::from_text(&written.unwrap().to_text());
    assert_eq!(again.file_layouts.inspector, FileLayout::Tree);
    let mut state = State::new(again, app::settings::Settings::default(), None);
    let _ = support::run(
        &mut state,
        Message::Started {
            folders: vec![repository.path().to_path_buf()],
        },
    );
    let _ = select(&mut state, "Explain the timeline");
    let tree = drawn(&state);
    let _directory = find(&tree, Role::Button, "src/app");
}

#[test]
fn details_show__the_button_beside_the_close_button_opens_the_page_of_the_commit() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let commit = select(&mut state, "Show the branch rail");
    let intents = Window::new(ThemeName::Dark, WIDE).click(&state, Role::Button, "Open as a page, with its changes");
    assert_eq!(intents, [Intent::Details(DetailsIntent::OpenPage(commit.clone()))]);
    ask(&mut state, intents[0].clone());
    let Some(Page::Commit(id)) = state.page_shown() else {
        panic!("the page of the commit is shown");
    };
    let page = state.commit_pages.pages.iter().find(|page| page.id == id).unwrap();
    assert_eq!(page.commit, commit);
}

#[test]
fn details_show__while_loading_the_inspector_says_loading() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let commit = row_id(&state, "Show the branch rail");
    let select = GraphIntent::Select(Selected::Commit(commit));
    let _effects = app::store::reduce(&mut state, Message::Intent(Intent::Graph(select)));
    assert_eq!(state.inspector.read, InspectorRead::Loading);
    assert!(says(&drawn(&state), "Loading\u{2026}"));
}

#[test]
fn details_show__the_close_button_closes_the_inspector_and_a_read_that_comes_late_is_ignored() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let commit = row_id(&state, "Show the branch rail");
    let selection = GraphIntent::Select(Selected::Commit(commit.clone()));
    let effects = app::store::reduce(&mut state, Message::Intent(Intent::Graph(selection)));
    let Some(app::store::Effect::Start(job)) = effects
        .iter()
        .find(|effect| matches!(effect, app::store::Effect::Start(_)))
    else {
        panic!("the details are read");
    };
    let job = job.clone();
    ask(&mut state, Intent::Details(DetailsIntent::Close));
    assert_eq!(state.timeline.selected, None);
    let late = app::work::jobs::run(&job, &git::CancelToken::new());
    let _ = app::store::reduce(&mut state, late);
    assert_eq!(
        state.inspector.read,
        InspectorRead::Loading,
        "the read of a row not selected any more is not taken"
    );
    assert!(says(
        &drawn(&state),
        "Select a commit to see its message and the files it changed."
    ));
    let _ = select(&mut state, "Show the branch rail");
    let intents = Window::new(ThemeName::Dark, WIDE).click(&state, Role::Button, "Close the inspector");
    assert_eq!(intents, [Intent::Details(DetailsIntent::Close)]);
}

#[test]
fn details_show_more__the_text_shows_four_lines_at_most_and_show_more_only_when_more_is_hidden() {
    let text = "One\nTwo\nThree\nFour\nFive\nSix\nSeven\nEight";
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (cut, tree) = message(&mut window, (text, false), 300.0);
    assert!(has(&tree, Role::Button, "Show more"));
    let (whole, _) = message(&mut window, (text, true), 300.0);
    assert!(
        (whole - 2.0 * cut).abs() < 1.0,
        "four of the eight lines: {cut} of {whole}"
    );
    let (_, tree) = message(&mut window, ("One\nTwo\nThree\nFour", false), 300.0);
    assert!(!has(&tree, Role::Button, "Show more"), "four lines hide nothing");
}

#[test]
fn details_show_more__show_more_shows_the_whole_text_in_place_and_becomes_show_less() {
    let repository = repository_for_details();
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Explain the timeline");
    let intents = Window::new(ThemeName::Dark, WIDE).click(&state, Role::Button, "Show more");
    assert_eq!(intents, [Intent::Details(DetailsIntent::ShowMore(true))]);
    ask(&mut state, intents[0].clone());
    let tree = drawn(&state);
    assert!(has(&tree, Role::Button, "Show less"));
    assert!(says_in(&tree, "The inspector shows the row selected."));
    let scroll_bars = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::ScrollBar)
        .count();
    let mut cut = state.clone();
    cut.inspector.is_message_expanded = false;
    let before = drawn(&cut)
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::ScrollBar)
        .count();
    assert!(scroll_bars <= before + 1, "the text has no scroll bar of its own");
}

#[test]
fn details_show_more__a_wider_or_narrower_inspector_shows_or_hides_show_more() {
    let text = "The guide starts with the timeline, where each row is a commit and the labels name the branches, the \
                tags and the remote branches; the draft is on top, a merge folds into one row, and the inspector beside \
                the rows shows the one selected.";
    let mut window = Window::new(ThemeName::Dark, (1400, 520));
    let (_, narrow) = message(&mut window, (text, false), 200.0);
    assert!(has(&narrow, Role::Button, "Show more"));
    let (_, wide) = message(&mut window, (text, false), 1300.0);
    assert!(!has(&wide, Role::Button, "Show more"));
}

#[test]
fn details_show_more__selecting_another_commit_shows_its_message_cut_again() {
    let repository = repository_for_details();
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Explain the timeline");
    ask(&mut state, Intent::Details(DetailsIntent::ShowMore(true)));
    let _ = select(&mut state, "Change the readme");
    assert!(!state.inspector.is_message_expanded);
    let _ = select(&mut state, "Explain the timeline");
    assert!(has(&drawn(&state), Role::Button, "Show more"));
}

/// The message drawn alone at a width, and the height of its text with the tree.
fn message(window: &mut Window, (text, is_expanded): (&str, bool), width: f32) -> (f32, TreeUpdate) {
    let state = MessageState { text, is_expanded };
    let (_, tree) = window.frame(&[], |ui| {
        ui.clip(Rect::new(0.0, 0.0, width, 500.0), |ui| {
            views::details::show_more(&state, ui)
        })
    });
    let height = tree
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some(text) || node.value() == Some(text))
        .and_then(|(_, node)| node.bounds())
        .map_or(0.0, |bounds| bounds.y1 - bounds.y0);
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a window is far smaller than an f32 counts"
    )]
    (height as f32, tree)
}
