//! **Show in the timeline** from the page of a commit, with repositories made for each test: the commit selected in
//! the timeline of its repository, loaded when it is further back than the history loaded, the branches shown taken
//! away when they leave it out; a page opened from Quick search; and the pictures of a page with its diffs in each
//! theme.

use app::store::Message;
use model::CommitId;
use model::filter::BranchesShown;
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::commit_page::CommitPageIntent;
use views::details::DetailsIntent;
use views::graph::Selected;
use views::page::Page;
use views::search::{QuickChoice, QuickCommand, SearchIntent};
use views::text::search as words;
use views::{FilterIntent, Intent};

use super::commit_page::{PAGE, ask_page, open, repository_for_page, shown_page};
use super::histories::{imported, line};
use super::inspector::{ask, row_id, select};
use super::search::{commit_of, search};
use super::support::{self, Window, find, says};
use super::{repository_with_history, showing};

#[test]
fn commit_page_show_in_graph__show_in_the_timeline_selects_the_commit_in_the_timeline_shown_in_place_of_the_page() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let commit = row_id(&state, "Read the settings");
    let id = open(&mut state, "Read the settings");
    let intents = Window::new(ThemeName::Dark, PAGE).click(&state, Role::Button, "Show in the timeline");
    assert_eq!(intents, [Intent::CommitPage(id, CommitPageIntent::ShowInTimeline)]);
    ask(&mut state, intents[0].clone());
    assert_eq!(state.page_shown(), None);
    assert_eq!(state.timeline.selected, Some(Selected::Commit(commit)));
    assert_eq!(state.pages.open, [Page::Commit(id)], "the page stays open");
}

#[test]
fn commit_page_show_in_graph__from_a_page_of_another_repository_the_window_is_pointed_back_at_it_first() {
    let first = Repository::new();
    let mut second = Repository::new();
    let _ = second.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let mut first = first;
    let _ = first.commit("Add the guide", &[("guide.md", "The timeline\n")]);
    let mut state = super::window();
    let folders = vec![first.path().to_path_buf(), second.path().to_path_buf()];
    let _ = support::run(&mut state, Message::Started { folders });
    ask(&mut state, Intent::SelectRepository(first.path().to_path_buf()));
    let commit = row_id(&state, "Add the guide");
    let id = open(&mut state, "Add the guide");
    ask(&mut state, Intent::SelectRepository(second.path().to_path_buf()));
    assert!(app::persisted::same_folder(
        state.shown_folder().unwrap(),
        second.path()
    ));
    ask(&mut state, Intent::ShowPage(Page::Commit(id)));
    ask_page(&mut state, id, CommitPageIntent::ShowInTimeline);
    assert!(app::persisted::same_folder(state.shown_folder().unwrap(), first.path()));
    assert_eq!(state.page_shown(), None);
    assert_eq!(state.timeline.selected, Some(Selected::Commit(commit)));
}

#[test]
fn commit_page_show_in_graph__a_commit_further_back_than_the_history_loaded_is_loaded_and_selected() {
    let repository = imported(&line(150));
    let mut state = super::window();
    state.settings.page_size = model::graph::PageSize::from_setting(Some(100.0));
    let _ = support::run(
        &mut state,
        Message::Started {
            folders: vec![repository.path().to_path_buf()],
        },
    );
    let first = repository.git(&["rev-list", "--max-parents=0", "HEAD"]);
    let commit = CommitId::parse(first.trim_end()).unwrap();
    assert!(!state.timeline.layout.shown_of.contains_key(&commit), "not loaded yet");
    ask(&mut state, Intent::Details(DetailsIntent::OpenPage(commit.clone())));
    let id = shown_page(&state);
    ask_page(&mut state, id, CommitPageIntent::ShowInTimeline);
    assert!(state.timeline.layout.shown_of.contains_key(&commit), "loaded");
    assert_eq!(state.timeline.selected, Some(Selected::Commit(commit)));
}

#[test]
fn show_in_the_timeline_on_a_commit_the_branches_shown_leave_out_shows_all_branches_again() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let commit = commit_of(&repository, "main");
    let id = open(&mut state, "Show the branch rail");
    let only_feature = BranchesShown::References(vec![String::from("refs/heads/feature")]);
    ask(&mut state, Intent::Filter(FilterIntent::Show(Some(only_feature))));
    assert!(
        !state.timeline.layout.shown_of.contains_key(&commit),
        "the branches shown leave it out"
    );
    ask(&mut state, Intent::ShowPage(Page::Commit(id)));
    ask_page(&mut state, id, CommitPageIntent::ShowInTimeline);
    assert_eq!(state.timeline.shown, None);
    assert_eq!(state.timeline.selected, Some(Selected::Commit(commit)));
}

/// Opens **Open commit** from Quick search, as choosing **Open Commit as a Page…** does, and types a name in it.
fn open_commit_typed(state: &mut app::store::State, name: &str) {
    search(state, SearchIntent::OpenQuick(String::from("open commit")));
    search(
        state,
        SearchIntent::ChooseQuick(QuickChoice::Command(QuickCommand::OpenCommitAsPage)),
    );
    search(state, SearchIntent::GoToCommitTyped(name.to_owned()));
}

#[test]
fn commit_page_open__typing_head_3_in_the_command_of_quick_search_opens_the_commit_three_before_head() {
    let repository = imported(&line(10));
    let mut state = showing(repository.path().to_path_buf());
    open_commit_typed(&mut state, "HEAD~3");
    let mut window = Window::new(ThemeName::Dark, PAGE);
    let (_, tree) = window.draw(&state, &[]);
    let _dialog = find(&tree, Role::Dialog, words::OPEN_COMMIT_TITLE);
    assert!(says(&tree, words::GO_TO_COMMIT_DETAIL), "it asks for a commit");
    let intents = window.click(&state, Role::Button, words::OPEN_PAGE);
    assert_eq!(intents, [Intent::Search(SearchIntent::GoToCommitSubmit)]);
    ask(&mut state, intents[0].clone());
    assert_eq!(state.search.dialog, None);
    let id = shown_page(&state);
    let wanted = commit_of(&repository, "HEAD~3");
    let page = state.commit_pages.pages.iter().find(|page| page.id == id).unwrap();
    assert_eq!(page.commit, wanted);
    assert_eq!(state.timeline.selected, None, "the timeline is not gone to");
}

#[test]
fn commit_page_open__a_name_git_does_not_know_is_refused_in_the_box_and_nothing_opens() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    open_commit_typed(&mut state, "nope");
    let (_, tree) = Window::new(ThemeName::Dark, PAGE).draw(&state, &[]);
    assert!(says(&tree, words::UNKNOWN_COMMIT));
    search(&mut state, SearchIntent::GoToCommitSubmit);
    assert!(state.search.dialog.is_some(), "the box stays open");
    assert_eq!(state.commit_pages.pages.len(), 0, "no page opens");
    assert_eq!(state.page_shown(), None);
}

#[test]
fn the_page_of_a_commit_with_its_diffs_is_drawn_in_each_theme() {
    let repository = repository_for_page();
    let mut state = showing(repository.path().to_path_buf());
    let id = open(&mut state, "Explain the timeline");
    ask_page(&mut state, id, CommitPageIntent::ShowChanges(true));
    ask_page(&mut state, id, CommitPageIntent::ShowFile(1));
    for (theme, name) in [
        (ThemeName::Dark, "commit-page-dark"),
        (ThemeName::Light, "commit-page-light"),
    ] {
        let mut window = Window::new(theme, PAGE);
        let _ = window.draw(&state, &[]);
        window.now += std::time::Duration::from_secs(2);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(name);
    }
}

#[test]
fn the_head_of_the_page_of_a_commit_is_drawn() {
    let repository = repository_for_page();
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Explain the timeline");
    let _ = open(&mut state, "Explain the timeline");
    let mut window = Window::new(ThemeName::Light, PAGE);
    let _ = window.draw(&state, &[]);
    window.assert_snapshot("commit-page-head-light");
}
