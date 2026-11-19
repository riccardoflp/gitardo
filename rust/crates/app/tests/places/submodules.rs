//! The Submodules list of the branch rail drawn from the store, with repositories whose submodules are cloned from
//! other throwaway repositories on disk (`submodule.section`): what a row shows, the list missing without submodules,
//! the double click that opens one in a new window, the icons of where each stands in every theme
//! (`submodule.status`), and the rows following a commit, a checkout or a file written in a submodule from a terminal
//! (`submodule.follow-changes`).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Duration;

use app::store::{Effect, Message, State};
use model::CommitId;
use model::contract::{FeatureId, OpenIn, Target};
use model::refs::{References, SortOrder};
use model::state::RepositoryState;
use model::submodule::{Checkout, Distance, Submodule, SubmoduleStatus, status};
use platform::accesskit::{Node, Role, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use views::feature::FeatureIntent;
use views::refs::{ReferencesState, StashesShown, tree_view};
use views::{Intent, RailIntent, RailList, RailLists};

use super::showing;
use super::support::{OperationsDriver, Window, has, run, says};
use super::watch::{changes_within, reads_after, watched};

/// The size of the window of these tests.
const WIDE: (u32, u32) = (1000, 520);
/// A window tall enough for the rows of the repository and its submodule above the rail, and the Submodules list in
/// it.
const TALL: (u32, u32) = (1000, 720);

/// The row of `lib` in the branch rail, rather than its row under the repository in **Repositories**.
fn lib_in_rail(tree: &TreeUpdate) -> &Node {
    let rail = super::pointer::bounds(tree, Role::Navigation, views::text::rail::RAIL);
    tree.nodes
        .iter()
        .map(|(_, node)| node)
        .filter(|node| node.role() == Role::Button && node.label() == Some("lib"))
        .find(|node| {
            node.bounds()
                .is_some_and(|bounds| super::pointer::rect(bounds).y >= rail.y)
        })
        .unwrap_or_else(|| panic!("no lib in the branch rail"))
}

/// A repository of its own with one commit, to be added as a submodule.
fn library() -> Repository {
    let mut library = Repository::new();
    let _commit = library.commit("Add the library", &[("lib.txt", "1\n")]);
    library
}

/// Adds a repository as a submodule in a folder, allowed to clone from a folder on disk for this command only.
fn add(repository: &Repository, library: &Repository, folder: &str) {
    let url = library.path().to_string_lossy().replace('\\', "/");
    let _added = repository.git(&[
        "-c",
        "protocol.file.allow=always",
        "submodule",
        "add",
        "--quiet",
        &url,
        folder,
    ]);
}

/// A repository with the submodule `lib`, committed, with a submodule `inner` inside it when asked.
pub(super) fn with_lib(has_inner: bool) -> (Repository, Vec<Repository>) {
    let inner = has_inner.then(library);
    let mut library = library();
    let mut kept = Vec::new();
    if let Some(inner) = inner {
        add(&library, &inner, "inner");
        let _with_inner = library.commit("Add inner", &[]);
        kept.push(inner);
    }
    let mut repository = Repository::new();
    let _first = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    add(&repository, &library, "lib");
    if has_inner {
        let _inner = repository.git(&[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "update",
            "--init",
            "--recursive",
            "--quiet",
        ]);
    }
    let _committed = repository.commit("Add the library", &[]);
    kept.push(library);
    (repository, kept)
}

/// A window showing a repository, with the Submodules list open.
fn with_submodules_open(repository: &Repository) -> State {
    let mut state = showing(repository.path().to_path_buf());
    let _effects = run(
        &mut state,
        Message::Intent(Intent::Rail(RailIntent::Toggle(RailList::Submodules))),
    );
    state
}

/// The submodule in a folder, as the window last read it.
pub(super) fn read_submodule(state: &State, folder: &str) -> Submodule {
    let read = state.shown_read().unwrap();
    read.state
        .submodules
        .iter()
        .find(|submodule| submodule.folder == folder)
        .cloned()
        .unwrap()
}

#[test]
fn submodule_section__lib_two_commits_ahead_on_main_with_a_file_changed_shows_lib_an_arrow_up_with_2_main_and_a_pencil_with_1()
 {
    let (repository, _kept) = with_lib(false);
    let lib = repository.path().join("lib");
    let _branch = repository.git(&["-C", "lib", "checkout", "--quiet", "-B", "main"]);
    for message in ["First change", "Second change"] {
        let _ = repository.git(&["-C", "lib", "commit", "--quiet", "--allow-empty", "-m", message]);
    }
    std::fs::write(lib.join("lib.txt"), "changed\n").unwrap();
    let state = with_submodules_open(&repository);
    let mut window = Window::new(ThemeName::Dark, TALL);
    let (_, tree) = window.draw(&state, &[]);
    let row = lib_in_rail(&tree);
    let value = row.value().unwrap_or_default();
    for said in ["main", "1 file not committed", "2 commits ahead of the recorded commit"] {
        assert!(value.contains(said), "{said}: {value}");
    }
    assert!(
        says(&tree, "2") && says(&tree, "1"),
        "the numbers beside the arrow and the pencil"
    );
}

#[test]
fn submodule_section__a_repository_without_submodules_has_no_submodules_list() {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a\n")]);
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::Button, views::text::rail::SUBMODULES));
    let (with, _kept) = with_lib(false);
    let mut with_submodule = showing(with.path().to_path_buf());
    super::fold_changes(&mut with_submodule);
    let (_, tree) = window.draw(&with_submodule, &[]);
    assert!(has(&tree, Role::Button, views::text::rail::SUBMODULES));
}

#[test]
fn submodule_open_in_new_window__a_double_click_on_lib_opens_a_window_on_its_folder() {
    let (repository, _kept) = with_lib(false);
    let mut state = with_submodules_open(&repository);
    let mut window = Window::new(ThemeName::Dark, TALL);
    let (_, tree) = window.draw(&state, &[]);
    let row = super::pointer::rect(lib_in_rail(&tree).bounds().unwrap());
    let intents = window.click_at(&state, super::pointer::middle(row), 2);
    let opens = Intent::Feature(FeatureIntent::Run(
        FeatureId("submodule.open-in-new-window"),
        Target::Submodule(String::from("lib")),
    ));
    assert!(intents.contains(&opens), "{intents:?}");
    let driver = OperationsDriver::new();
    driver.intent(&mut state, opens);
    let effects = loop {
        let before = state.clone();
        let message = driver.next(&mut state);
        if matches!(message, Message::FeatureEnded { .. }) {
            break run(&mut before.clone(), message);
        }
    };
    assert!(
        effects.contains(&Effect::OpenWindow(repository.path().join("lib"))),
        "{effects:?}"
    );
}

/// A commit of a test, by a digit.
fn commit(digit: char) -> CommitId {
    CommitId::parse(&digit.to_string().repeat(40)).unwrap()
}

/// A submodule by hand, in each of the states of `submodule.status`.
fn made(folder: &str, checkout: Option<(Option<Distance>, bool)>, has_record: bool) -> Submodule {
    Submodule {
        name: folder.to_owned(),
        folder: folder.to_owned(),
        parent: None,
        url: None,
        branch: None,
        recorded: has_record.then(|| commit('1')),
        staged: None,
        checkout: checkout.map(|(distance, is_on_it)| Checkout {
            commit: Some(if is_on_it { commit('1') } else { commit('2') }),
            branch: Some(String::from("main")),
            uncommitted: 0,
            distance,
        }),
    }
}

/// One submodule in each state, in the order of the table of `submodule.status`.
fn every_status() -> Vec<Submodule> {
    let distance = |ahead, behind| Some(Distance { ahead, behind });
    let all = vec![
        made("not-initialized", None, true),
        made("added", Some((None, false)), false),
        made("unknown", Some((None, false)), true),
        made("both", Some((distance(2, 1), false)), true),
        made("ahead", Some((distance(2, 0), false)), true),
        made("behind", Some((distance(0, 1), false)), true),
        made("on-it", Some((distance(0, 0), true)), true),
    ];
    let statuses: Vec<SubmoduleStatus> = all.iter().map(status).collect();
    assert!(matches!(
        statuses.as_slice(),
        [
            SubmoduleStatus::NotInitialized,
            SubmoduleStatus::Added,
            SubmoduleStatus::Unknown { .. },
            SubmoduleStatus::Both { .. },
            SubmoduleStatus::Ahead { .. },
            SubmoduleStatus::Behind { .. },
            SubmoduleStatus::OnIt { .. }
        ]
    ));
    all
}

#[test]
fn submodule_status__the_icons_read_in_the_dark_the_light_and_both_high_contrast_themes() {
    let repository = RepositoryState {
        submodules: every_status(),
        ..RepositoryState::default()
    };
    let references = References::default();
    let mut lists = RailLists::default();
    let _ = lists.turned.insert(RailList::Submodules);
    let (unfolded, files) = (BTreeSet::new(), BTreeMap::new());
    let colors = HashMap::new();
    let state = ReferencesState {
        tag_marks: None,
        repository: &repository,
        references: &references,
        lists: &lists,
        branch_colors: &colors,
        remote_colors: &colors,
        favorites: &[],
        recent: &[],
        typed: "",
        shown: None,
        list_filter: views::refs::ListFilterState {
            listed: None,
            references: &references,
            open: None,
        },
        selected: None,
        sort: SortOrder::Name,
        open_in: OpenIn::Checkout,
        window: views::worktree::WorktreeWindow::default(),
        github: views::github::GitHubShown::default(),
        stashes: StashesShown {
            unfolded: &unfolded,
            files: &files,
            is_operation_running: false,
            now: 0,
        },
    };
    for (theme, name) in [
        (ThemeName::Dark, "submodules-dark"),
        (ThemeName::Light, "submodules-light"),
        (ThemeName::HighContrastDark, "submodules-high-contrast-dark"),
        (ThemeName::HighContrastLight, "submodules-high-contrast-light"),
    ] {
        // The rail lists names from 900 pixels on; it is drawn as wide as in a window that wide.
        let mut window = Window::new(theme, (900, 440));
        let rail = ui::Rect::new(0.0, 0.0, 300.0, 440.0);
        let _ = window.frame(&[], |ui| ui.clip(rail, |ui| tree_view(&state, ui)));
        window.assert_snapshot(name);
    }
}

/// The repository read again after what the watcher saw within a moment, once the reads it asks for are due.
fn read_after_watching(state: &mut State, receiver: &std::sync::mpsc::Receiver<Message>) {
    let changes = changes_within(receiver, Duration::from_millis(1_500));
    assert!(reads_after(state, changes) > 0, "a read is asked for");
}

#[test]
fn submodule_follow_changes__a_commit_made_in_lib_from_a_terminal_turns_its_icon_into_an_arrow_up_with_1() {
    let (repository, _kept) = with_lib(false);
    let folder = repository.path().to_path_buf();
    let mut state = showing(folder.clone());
    let (watch, receiver) = watched(&folder);
    let _ = repository.git(&[
        "-C",
        "lib",
        "commit",
        "--quiet",
        "--allow-empty",
        "-m",
        "Change the library",
    ]);
    read_after_watching(&mut state, &receiver);
    assert!(matches!(
        status(&read_submodule(&state, "lib")),
        SubmoduleStatus::Ahead { commits: 1, .. }
    ));
    drop(watch);
}

#[test]
fn submodule_follow_changes__a_checkout_of_another_branch_in_lib_inner_from_a_terminal_shows_that_branch() {
    let (repository, _kept) = with_lib(true);
    let folder = repository.path().to_path_buf();
    let mut state = showing(folder.clone());
    assert_eq!(
        read_submodule(&state, "lib/inner")
            .checkout
            .and_then(|checkout| checkout.branch),
        None
    );
    let (watch, receiver) = watched(&folder);
    let _ = repository.git(&["-C", "lib/inner", "checkout", "--quiet", "-b", "other"]);
    read_after_watching(&mut state, &receiver);
    let inner = read_submodule(&state, "lib/inner");
    assert_eq!(
        inner.checkout.and_then(|checkout| checkout.branch).as_deref(),
        Some("other")
    );
    drop(watch);
}

#[test]
fn submodule_follow_changes__a_file_changed_in_lib_changes_its_pencil_count() {
    let (repository, _kept) = with_lib(false);
    let folder = repository.path().to_path_buf();
    let mut state = showing(folder.clone());
    let (watch, receiver) = watched(&folder);
    std::fs::write(folder.join("lib").join("lib.txt"), "changed\n").unwrap();
    read_after_watching(&mut state, &receiver);
    assert_eq!(
        read_submodule(&state, "lib")
            .checkout
            .map(|checkout| checkout.uncommitted),
        Some(1)
    );
    drop(watch);
}
