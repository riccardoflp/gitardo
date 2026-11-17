//! What is done from a comparison: a side changed, a file taken from the second side, the comparison copied as a
//! patch; and the comparison of two commits in the inspector.

use app::store::feature::Ended;
use app::store::{Message, State};
use model::CommitId;
use model::contract::{Answers, FeatureId, FollowUp, Target};
use model::state::ChangeKind;
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::compare::CompareIntent;
use views::diff::DiffVersion;
use views::feature::FeatureIntent;
use views::page::Page;

use super::compare::{compared, page_id, repository, until_ended};
use super::showing;
use super::support::{OperationsDriver, Window, has, says, says_in};

/// What the window shows.
fn drawn(state: &State) -> TreeUpdate {
    Window::new(ThemeName::Dark, (1200, 900)).draw(state, &[]).1
}

/// Clicks a button of the window, and reduces what it asks for.
fn click(driver: &OperationsDriver, state: &mut State, name: &str) {
    let intents = Window::new(ThemeName::Dark, (1200, 900)).click(state, Role::Button, name);
    for intent in intents {
        driver.intent(state, intent);
    }
}

/// Clicks a button of the comparison page, the rightmost of that name: the branch rail beside it has one too.
fn click_in_page(driver: &OperationsDriver, state: &mut State, name: &str) {
    let mut window = Window::new(ThemeName::Dark, (1200, 900));
    let (_, tree) = window.draw(state, &[]);
    let at = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some(name))
        .filter_map(|(_, node)| node.bounds().map(super::pointer::rect))
        .max_by(|one, other| one.x.total_cmp(&other.x))
        .unwrap_or_else(|| panic!("no button {name}"));
    for intent in super::pointer::click_at(&mut window, state, super::pointer::middle(at), 1) {
        driver.intent(state, intent);
    }
}

/// Waits until the operation running ends, confirming its plan, and gives what it did after.
fn after_it_ended(driver: &OperationsDriver, state: &mut State) -> Vec<FollowUp> {
    loop {
        match driver.next(state) {
            Message::FeatureConfirms(_) => driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureEnded {
                ended: Ended::Ran { after, .. },
                ..
            } => return after,
            Message::FeatureEnded { ended, .. } => panic!("{ended:?}"),
            _ => {}
        }
    }
}

/// The patch **Copy as patch** puts on the clipboard.
fn copied(driver: &OperationsDriver, state: &mut State) -> String {
    click(driver, state, "Copy as patch");
    after_it_ended(driver, state)
        .into_iter()
        .find_map(|follow_up| match follow_up {
            FollowUp::CopyToClipboard(text) => Some(text),
            _ => None,
        })
        .unwrap()
}

/// The commit a revision stands for.
fn commit(repository: &Repository, revision: &str) -> CommitId {
    CommitId::parse(repository.git(&["rev-parse", revision]).trim()).unwrap()
}

/// A window showing, in the inspector, the comparison `compare.with-current` opens on `feature`, `main` checked out.
fn in_place(repository: &Repository) -> (State, OperationsDriver) {
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let with_current = FeatureIntent::Run(
        FeatureId("compare.with-current"),
        Target::Branch(String::from("feature")),
    );
    driver.intent(&mut state, Intent::Feature(with_current));
    until_ended(&driver, &mut state);
    (state, driver)
}

/// The files of the comparison in the inspector, with how each changed.
fn in_place_files(state: &State) -> Vec<(String, ChangeKind)> {
    let in_place = state.compares.in_place.as_ref().unwrap();
    let files = in_place.files.as_ref().unwrap().as_ref().unwrap();
    files.iter().map(|file| (file.path.clone(), file.kind)).collect()
}

#[test]
fn compare_change_side__choosing_release_in_place_of_main_compares_it_with_feature_in_the_same_page_retitled() {
    let repository = repository();
    repository.branch("release");
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let id = page_id(&state);
    click_in_page(&driver, &mut state, "main");
    assert!(state.compares.chooser.is_some(), "the chooser is open");
    click_in_page(&driver, &mut state, "release");
    click_in_page(&driver, &mut state, "Compare");
    until_ended(&driver, &mut state);
    assert_eq!(page_id(&state), id);
    assert_eq!(state.compares.pages.len(), 1);
    assert!(says(&drawn(&state), "release \u{2194} feature"));
    assert!(has(&drawn(&state), Role::Button, "release"));
}

#[test]
fn compare_take_file__taking_c_from_feature_runs_git_checkout_after_asking_and_stages_its_contents() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    click(&driver, &mut state, "Take the version of feature into the working tree");
    let _after = after_it_ended(&driver, &mut state);
    let feature = commit(&repository, "feature");
    let ran: Vec<String> = state.log.entries().map(|entry| entry.line.clone()).collect();
    assert!(ran.contains(&format!("git checkout {feature} -- c.txt")), "{ran:?}");
    assert_eq!(repository.git(&["diff", "--cached", "--name-only"]).trim(), "c.txt");
    assert_eq!(std::fs::read_to_string(repository.path().join("c.txt")).unwrap(), "c\n");
}

#[test]
fn compare_take_file__the_arrow_is_not_on_the_files_of_a_comparison_whose_second_side_is_the_working_tree() {
    let repository = repository();
    std::fs::write(repository.path().join("a.txt"), "changed\n").unwrap();
    let (state, _driver) = compared(&repository, ("ref:feature", "working-tree"));
    let tree = drawn(&state);
    assert!(says_in(&tree, "a.txt"));
    let arrows = tree
        .nodes
        .iter()
        .filter_map(|(_, node)| node.label())
        .filter(|label| {
            label.starts_with("Take the version of") || label.starts_with("Delete it from the working tree")
        })
        .count();
    assert_eq!(arrows, 0);
}

#[test]
fn compare_copy_patch__the_patch_applied_on_the_first_side_gives_the_files_of_the_second_in_everything_that_differs() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let id = page_id(&state);
    let everything = CompareIntent::SetMode(id, model::compare::CompareMode::Everything);
    driver.intent(&mut state, Intent::Compare(everything));
    let patch = copied(&driver, &mut state);
    let file = repository.path().join(".git").join("copied.patch");
    std::fs::write(&file, patch).unwrap();
    let _applied = repository.git(&["apply", "--index", file.to_str().unwrap()]);
    assert_eq!(repository.git(&["diff", "--cached", "feature", "--name-only"]), "");
}

#[test]
fn compare_copy_patch__a_binary_file_changed_is_in_the_patch() {
    let mut repository = repository();
    repository.checkout("feature");
    let _binary = repository.commit("Add a picture", &[("picture.bin", "\u{0}\u{1}\u{2}binary\u{0}")]);
    repository.checkout("main");
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let patch = copied(&driver, &mut state);
    assert!(patch.contains("diff --git a/picture.bin b/picture.bin"), "{patch}");
    assert!(patch.contains("GIT binary patch"), "{patch}");
}

#[test]
fn compare_copy_patch__narrowed_to_a_folder_only_its_files_are_in_the_patch() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let id = page_id(&state);
    driver.intent(
        &mut state,
        Intent::Compare(CompareIntent::TypePath(id, String::from("src"))),
    );
    driver.intent(&mut state, Intent::Compare(CompareIntent::Narrow(id)));
    let patch = copied(&driver, &mut state);
    let headers: Vec<&str> = patch.lines().filter(|line| line.starts_with("diff --git ")).collect();
    assert_eq!(headers, ["diff --git a/src/b.txt b/src/b.txt"]);
}

#[test]
fn compare_copy_patch__the_message_counts_the_files_of_the_patch() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let _patch = copied(&driver, &mut state);
    let said = state.operations.said.as_ref().unwrap();
    assert_eq!(said.text, "Copied the changes of 2 files as a patch.");
}

#[test]
fn compare_with_current__on_feature_with_main_checked_out_the_inspector_shows_the_files_from_feature_to_main() {
    let repository = repository();
    let (state, _driver) = in_place(&repository);
    let files = in_place_files(&state);
    assert_eq!(
        files,
        [
            (String::from("c.txt"), ChangeKind::Deleted),
            (String::from("d.txt"), ChangeKind::Added),
            (String::from("src/b.txt"), ChangeKind::Deleted),
        ]
    );
    let tree = drawn(&state);
    assert!(says(&tree, "3 changed files"));
    assert!(has(&tree, Role::Button, "d.txt"));
}

#[test]
fn compare_selected_before__the_files_from_the_commit_selected_before_to_the_one_clicked_are_listed_in_the_inspector() {
    let repository = repository();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let (before, clicked) = (commit(&repository, "main~1"), commit(&repository, "feature"));
    let target = Target::Commits(vec![before.clone(), clicked.clone()]);
    driver.intent(
        &mut state,
        Intent::Feature(FeatureIntent::Run(FeatureId("compare.selected-before"), target)),
    );
    until_ended(&driver, &mut state);
    let sides = &state.compares.in_place.as_ref().unwrap().sides;
    assert_eq!((&sides.from.commit, &sides.to.commit), (&before, &clicked));
    assert_eq!(sides.from.name, before.short());
    let files: Vec<String> = in_place_files(&state).into_iter().map(|(path, _)| path).collect();
    assert_eq!(files, ["c.txt", "src/b.txt"]);
    assert!(says(&drawn(&state), "2 changed files"));
}

#[test]
fn compare_in_place__swap_shows_the_files_changed_from_the_second_to_the_first_the_names_exchanged() {
    let repository = repository();
    let (mut state, driver) = in_place(&repository);
    click(&driver, &mut state, "Swap");
    let sides = &state.compares.in_place.as_ref().unwrap().sides;
    assert_eq!((sides.from.name.as_str(), sides.to.name.as_str()), ("main", "feature"));
    assert!(in_place_files(&state).contains(&(String::from("d.txt"), ChangeKind::Deleted)));
}

#[test]
fn compare_in_place__open_as_a_page_opens_the_comparison_page_of_the_same_two_sides_named_as_here() {
    let repository = repository();
    let (mut state, driver) = in_place(&repository);
    click(&driver, &mut state, "Open as a page");
    let id = page_id(&state);
    let page = state.compares.pages.iter().find(|page| page.id == id).unwrap();
    let names = (page.target.first.name.as_str(), page.target.second.name.as_str());
    assert_eq!(names, ("feature", "main"));
}

#[test]
fn compare_in_place__a_click_on_a_file_opens_its_diff_page_between_the_two_commits() {
    let repository = repository();
    let (mut state, driver) = in_place(&repository);
    click(&driver, &mut state, "d.txt");
    assert!(
        matches!(state.page_shown(), Some(Page::Diff(_))),
        "{:?}",
        state.page_shown()
    );
    let page = state.diff_pages.pages.last().unwrap();
    let side = |name: &str| DiffVersion::Commit {
        commit: commit(&repository, name),
        name: name.to_owned(),
    };
    assert_eq!(
        (&page.target.left, &page.target.right),
        (&side("feature"), &side("main"))
    );
}

#[test]
fn compare_in_place__closing_it_shows_the_card_of_the_commit_selected_again() {
    let repository = repository();
    let (mut state, driver) = in_place(&repository);
    assert!(has(&drawn(&state), Role::Button, "Close the comparison"));
    driver.intent(&mut state, Intent::Feature(FeatureIntent::Dismiss));
    click(&driver, &mut state, "Close the comparison");
    assert_eq!(state.compares.in_place, None);
    let labels = |tree: &TreeUpdate| -> Vec<String> {
        tree.nodes
            .iter()
            .filter_map(|(_, node)| node.label().map(str::to_owned))
            .collect()
    };
    let before = drawn(&showing(repository.path().to_path_buf()));
    let tree = drawn(&state);
    assert!(!has(&tree, Role::Button, "Close the comparison"));
    assert_eq!(labels(&tree), labels(&before));
}

#[test]
fn compare_in_place__asked_from_the_branch_rail_the_comparison_shows_in_the_inspector_too() {
    let repository = repository();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let rail = super::support::Window::new(ThemeName::Dark, (1200, 900))
        .draw(&state, &[])
        .1;
    assert!(has(&rail, Role::TreeItem, "feature") || says_in(&rail, "feature"));
    let with_current = FeatureIntent::RunWith(
        FeatureId("compare.with-current"),
        Target::Branch(String::from("feature")),
        Answers::new(),
    );
    driver.intent(&mut state, Intent::Feature(with_current));
    until_ended(&driver, &mut state);
    assert_eq!(state.page_shown(), None);
    assert!(has(&drawn(&state), Role::Button, "Close the comparison"));
}

#[test]
fn compare_open_on_github__the_button_of_main_and_feature_tracking_origin_on_github_opens_main_three_dots_feature() {
    let repository = repository();
    let _ = repository.git(&["remote", "add", "origin", "https://github.com/owner/app.git"]);
    for branch in ["main", "feature"] {
        let _ = repository.git(&["update-ref", &format!("refs/remotes/origin/{branch}"), branch]);
        let _ = repository.git(&["branch", &format!("--set-upstream-to=origin/{branch}"), branch]);
    }
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    assert!(
        !has(&drawn(&state), Role::Button, "Open on GitHub"),
        "not before GitHub is known"
    );
    let detect = app::store::github::GitHubJob {
        id: state.github.detecting.unwrap(),
        folder: state.github.folder.clone().unwrap(),
        work: app::store::github::GitHubWork::Detect,
    };
    let access = app::work::github::GitHubAccess::system();
    let _ = super::support::run(&mut state, app::work::github::run(&access, &detect));
    click(&driver, &mut state, "Open on GitHub");
    let url = String::from("https://github.com/owner/app/compare/main...feature");
    loop {
        let message = driver.raw_next();
        let has_ended = matches!(message, Message::FeatureEnded { .. });
        let effects = super::support::run(&mut state, message);
        driver.take(&state, &effects);
        if has_ended {
            assert!(effects.contains(&app::store::Effect::OpenUrl(url)), "{effects:?}");
            break;
        }
    }
    assert_eq!(repository.git(&["status", "--porcelain"]), "", "nothing changes");
}
