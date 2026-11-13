//! The Merge page drawn from the store, its file in conflict read and computed by the workers in repositories made for
//! each test: the labels and the regions of a stopped merge, the buttons that take a side, what needs no choice, the
//! base, the conflicts left and `F7`.

use app::store::{MergePage, Message, State};
use model::merge_view::{Choice, RegionKind, RegionState};
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use ui::Modifiers;
use views::Intent;
use views::merge_view::{MergeIntent, MergePageIntent, MergeText};
use views::page::Page;

use super::diff::{f7, press};
use super::inspector::ask;
use super::pointer::apply;
use super::showing;
use super::support::{Window, has, says, shortcut};

/// The size of the window of the Merge pages.
pub const MERGE: (u32, u32) = (1200, 700);
/// The file in conflict.
pub const FILE: &str = "src/app.rs";

/// Thirty lines, `line 1` to `line 30`, with some of them written another way.
pub fn lines_with(changed: &[(usize, &str)]) -> String {
    (1..=30)
        .map(|line| match changed.iter().find(|(place, _)| *place == line) {
            Some((_, text)) => format!("{text}\n"),
            None => format!("line {line}\n"),
        })
        .collect()
}

/// A merge of `feature` into `main` stopped on `src/app.rs`, whose 30 lines each branch changed as it says.
pub fn stopped_merge(on_main: &[(usize, &str)], on_feature: &[(usize, &str)]) -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the app", &[(FILE, &lines_with(&[]))]);
    repository.branch("feature");
    let _ = repository.commit("Change the app on main", &[(FILE, &lines_with(on_main))]);
    repository.checkout("feature");
    let _ = repository.commit("Change the app on feature", &[(FILE, &lines_with(on_feature))]);
    repository.checkout("main");
    stop_merging(&repository);
    repository
}

/// Merges `feature` into `main`, which stops on a conflict.
fn stop_merging(repository: &Repository) {
    let output = repository
        .command("git")
        .args(["merge", "--no-ff", "--no-edit", "feature"])
        .output()
        .unwrap();
    assert!(!output.status.success(), "the merge stops on a conflict");
}

/// A merge stopped on conflicts at some lines of `src/app.rs`, each `line N on main` against `line N on feature`.
pub fn conflicts_at(places: &[usize]) -> Repository {
    let main: Vec<(usize, String)> = places
        .iter()
        .map(|line| (*line, format!("line {line} on main")))
        .collect();
    let feature: Vec<(usize, String)> = places
        .iter()
        .map(|line| (*line, format!("line {line} on feature")))
        .collect();
    let main: Vec<(usize, &str)> = main.iter().map(|(line, text)| (*line, text.as_str())).collect();
    let feature: Vec<(usize, &str)> = feature.iter().map(|(line, text)| (*line, text.as_str())).collect();
    stopped_merge(&main, &feature)
}

/// A block of git's markers around two parts.
fn markers(current: &[&str], incoming: &[&str]) -> String {
    let mut block = String::from("<<<<<<< HEAD\n");
    for line in current {
        block.push_str(line);
        block.push('\n');
    }
    block.push_str("=======\n");
    for line in incoming {
        block.push_str(line);
        block.push('\n');
    }
    block.push_str(">>>>>>> feature\n");
    block
}

/// Writes `src/app.rs` in the working tree.
pub fn write(repository: &Repository, text: &str) {
    std::fs::write(repository.path().join("src").join("app.rs"), text).unwrap();
}

/// The file in conflict as it is on disk.
pub fn on_disk(repository: &Repository) -> String {
    std::fs::read_to_string(repository.path().join("src").join("app.rs")).unwrap()
}

/// A window showing the repository, with the Merge page of `src/app.rs` opened and read.
pub fn opened(repository: &Repository) -> State {
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state);
    state
}

/// Opens the Merge page of `src/app.rs`, and runs the read it starts.
pub fn open(state: &mut State) {
    ask(state, Intent::MergeView(MergeIntent::Open(FILE.to_owned())));
}

/// The Merge page shown, which the test fails without.
pub fn shown(state: &State) -> &MergePage {
    let Some(Page::Merge(id)) = state.page_shown() else {
        panic!("a Merge page is shown, not {:?}", state.page_shown());
    };
    state.merge_pages.pages.iter().find(|page| page.id == id).unwrap()
}

/// The texts and regions of the page shown.
pub fn text(state: &State) -> &MergeText {
    shown(state).text.as_deref().unwrap()
}

/// Asks something of the page shown, and runs the reads it starts.
pub fn page_intent(state: &mut State, intent: MergePageIntent) {
    let id = shown(state).id;
    ask(state, Intent::MergeView(MergeIntent::Page(id, intent)));
}

/// The first region that is a conflict, by its place.
pub fn first_conflict(state: &State) -> usize {
    text(state)
        .regions
        .iter()
        .position(|region| region.kind == RegionKind::Conflict)
        .unwrap()
}

/// How many nodes of a role and a name the tree has.
fn count(tree: &TreeUpdate, role: Role, name: &str) -> usize {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == role && node.label() == Some(name))
        .count()
}

/// Whether the result still has git's markers.
fn has_markers(result: &str) -> bool {
    result
        .lines()
        .any(|line| line.starts_with("<<<<<<<") || line.starts_with(">>>>>>>"))
}

#[test]
fn merge_view_page__during_a_merge_of_feature_into_main_the_columns_are_labelled_with_the_sides_and_the_result() {
    let repository = conflicts_at(&[10]);
    let state = opened(&repository);
    let (_, tree) = Window::new(ThemeName::Dark, MERGE).draw(&state, &[]);
    assert!(says(&tree, "Current \u{2014} The branch checked out"));
    assert!(says(&tree, "src/app.rs \u{2014} Result"));
    assert!(says(&tree, "Incoming \u{2014} The branch being merged"));
    assert_eq!(shown(&state).title, "app.rs (Current \u{2194} Incoming)");
}

#[test]
fn merge_view_page__a_change_made_only_on_main_is_a_current_region_with_no_band_on_the_incoming_side() {
    let repository = stopped_merge(
        &[(4, "line 4 on main"), (20, "line 20 on main")],
        &[(20, "line 20 on feature")],
    );
    let state = opened(&repository);
    let region = text(&state).regions.first().unwrap();
    assert_eq!((region.kind, region.state), (RegionKind::Current, RegionState::Current));
    let (_, tree) = Window::new(ThemeName::Dark, MERGE).draw(&state, &[]);
    assert_eq!(
        count(&tree, Role::Button, "Accept incoming"),
        1,
        "only the conflict offers the incoming side"
    );
}

#[test]
fn merge_view_page__a_conflict_has_bands_on_both_sides_and_covers_the_lines_of_its_markers() {
    let repository = conflicts_at(&[10]);
    let state = opened(&repository);
    let place = first_conflict(&state);
    let region = text(&state).regions.get(place).unwrap();
    assert!(region.is_unresolved());
    assert_eq!(region.result, 9..14, "the 5 lines from <<<<<<< to >>>>>>>");
    assert!(text(&state).result_line(9).starts_with("<<<<<<<"));
    let (_, tree) = Window::new(ThemeName::Dark, MERGE).draw(&state, &[]);
    assert!(has(&tree, Role::Button, "Accept current"));
    assert!(has(&tree, Role::Button, "Accept incoming"));
    assert!(says(&tree, "1 conflict left"));
}

#[test]
fn merge_view_page__a_conflict_whose_markers_were_deleted_by_hand_keeping_one_part_is_resolved_and_drawn_faintly() {
    let repository = conflicts_at(&[10]);
    write(&repository, &lines_with(&[(10, "line 10 on main")]));
    let state = opened(&repository);
    let region = text(&state).regions.get(first_conflict(&state)).unwrap();
    assert!(!region.is_unresolved());
    assert_eq!(region.state, RegionState::Current);
    assert_eq!(text(&state).conflicts_left(), 0);
}

#[test]
fn merge_view_page__a_file_that_both_sides_added_opens_with_an_empty_base() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    repository.branch("feature");
    let _ = repository.commit("Add the app on main", &[(FILE, "a\nB1\n")]);
    repository.checkout("feature");
    let _ = repository.commit("Add the app on feature", &[(FILE, "a\nB2\n")]);
    repository.checkout("main");
    stop_merging(&repository);
    let state = opened(&repository);
    assert_eq!(text(&state).base, "");
    assert_eq!(text(&state).conflicts_left(), 1);
}

#[test]
fn merge_view_take_side__accept_current_on_b1_against_b2_makes_the_result_b1_and_both_puts_b1_then_b2() {
    let repository = conflicts_at(&[10]);
    let mut state = opened(&repository);
    let mut window = Window::new(ThemeName::Dark, MERGE);
    let intents = window.click(&state, Role::Button, "Accept current");
    apply(&mut state, intents);
    assert_eq!(shown(&state).result, lines_with(&[(10, "line 10 on main")]));
    page_intent(&mut state, MergePageIntent::Undo);
    let place = first_conflict(&state);
    page_intent(&mut state, MergePageIntent::Take(place, Choice::CurrentFirst));
    let both = lines_with(&[]).replace("line 10\n", "line 10 on main\nline 10 on feature\n");
    assert_eq!(shown(&state).result, both);
    assert_eq!(on_disk(&repository), shown(&state).saved, "nothing is saved");
}

#[test]
fn merge_view_take_side__after_accept_current_its_button_is_gone_and_accept_incoming_is_still_there() {
    let repository = conflicts_at(&[10]);
    let mut state = opened(&repository);
    let place = first_conflict(&state);
    page_intent(&mut state, MergePageIntent::Take(place, Choice::Current));
    let (_, tree) = Window::new(ThemeName::Dark, MERGE).draw(&state, &[]);
    assert!(!has(&tree, Role::Button, "Accept current"));
    assert!(has(&tree, Role::Button, "Accept incoming"));
}

#[test]
fn merge_view_take_side__ctrl_z_after_accept_current_brings_back_the_markers_exactly() {
    let repository = conflicts_at(&[10]);
    let mut state = opened(&repository);
    let before = shown(&state).result.clone();
    let place = first_conflict(&state);
    page_intent(&mut state, MergePageIntent::Take(place, Choice::Current));
    assert!(!has_markers(&shown(&state).result));
    let mut window = Window::new(ThemeName::Dark, MERGE);
    let _ = press(&mut window, &mut state, &[shortcut('z', Modifiers::CONTROL)]);
    assert_eq!(shown(&state).result, before);
    let _ = press(&mut window, &mut state, &[shortcut('y', Modifiers::CONTROL)]);
    assert!(!has_markers(&shown(&state).result), "Ctrl+Y puts the choice again");
}

#[test]
fn merge_view_take_side__a_region_of_the_current_side_only_already_in_the_result_has_no_button() {
    let repository = stopped_merge(
        &[(4, "line 4 on main"), (20, "line 20 on main")],
        &[(20, "line 20 on feature")],
    );
    let state = opened(&repository);
    assert_eq!(text(&state).offers.first().map(|offers| offers.current), Some(false));
    let (_, tree) = Window::new(ThemeName::Dark, MERGE).draw(&state, &[]);
    assert_eq!(
        count(&tree, Role::Button, "Accept current"),
        1,
        "only the conflict offers the current side"
    );
    assert_eq!(count(&tree, Role::Button, "Accept incoming"), 1);
}

#[test]
fn merge_view_accept_simple__a_conflict_whose_two_parts_are_the_same_lines_gets_them_once_without_markers() {
    let repository = conflicts_at(&[10]);
    let same = markers(&["line 10 on main"], &["line 10 on main"]);
    write(&repository, &lines_with(&[]).replace("line 10\n", &same));
    let mut state = opened(&repository);
    page_intent(&mut state, MergePageIntent::AcceptSimple);
    assert_eq!(shown(&state).result, lines_with(&[(10, "line 10 on main")]));
}

#[test]
fn merge_view_accept_simple__a_conflict_b1_against_b2_is_left_as_it_is() {
    let repository = conflicts_at(&[5, 20]);
    let different = markers(&["line 5 on main"], &["line 5 on feature"]);
    let same = markers(&["line 20 on main"], &["line 20 on main"]);
    let written = lines_with(&[])
        .replace("line 5\n", &different)
        .replace("line 20\n", &same);
    write(&repository, &written);
    let mut state = opened(&repository);
    assert_eq!(text(&state).conflicts_left(), 2);
    page_intent(&mut state, MergePageIntent::AcceptSimple);
    assert_eq!(text(&state).conflicts_left(), 1);
    assert!(
        shown(&state)
            .result
            .contains("<<<<<<< HEAD\nline 5 on main\n=======\nline 5 on feature\n>>>>>>> feature\n")
    );
}

#[test]
fn merge_view_accept_simple__one_ctrl_z_after_it_brings_back_every_marker_it_removed() {
    let repository = conflicts_at(&[5, 20]);
    let five = markers(&["line 5 on main"], &["line 5 on main"]);
    let twenty = markers(&["line 20 on main"], &["line 20 on main"]);
    let written = lines_with(&[]).replace("line 5\n", &five).replace("line 20\n", &twenty);
    write(&repository, &written);
    let mut state = opened(&repository);
    page_intent(&mut state, MergePageIntent::AcceptSimple);
    assert_eq!(text(&state).conflicts_left(), 0);
    let mut window = Window::new(ThemeName::Dark, MERGE);
    let _ = press(&mut window, &mut state, &[shortcut('z', Modifiers::CONTROL)]);
    assert_eq!(shown(&state).result, written);
}

#[test]
fn merge_view_show_base__show_the_base_adds_the_fourth_column_and_hide_the_base_removes_it() {
    let repository = conflicts_at(&[10]);
    let mut state = opened(&repository);
    let mut window = Window::new(ThemeName::Dark, MERGE);
    let base = "Base \u{2014} What the two sides started from";
    let intents = window.click(&state, Role::Button, "Show the base");
    apply(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, base));
    assert!(shown(&state).layout.rows.iter().any(|row| row.base.is_some()));
    let intents = window.click(&state, Role::Button, "Hide the base");
    apply(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!says(&tree, base));
    assert!(shown(&state).layout.rows.iter().all(|row| row.base.is_none()));
}

#[test]
fn merge_view_show_base__after_showing_it_and_closing_the_page_the_next_merge_page_opens_with_the_base_shown() {
    let repository = conflicts_at(&[10]);
    let mut state = opened(&repository);
    let id = shown(&state).id;
    let effects = super::support::run(
        &mut state,
        Message::Intent(Intent::MergeView(MergeIntent::Page(
            id,
            MergePageIntent::ShowBase(true),
        ))),
    );
    let written = effects
        .iter()
        .any(|effect| matches!(effect, app::store::Effect::WritePersisted(persisted) if persisted.merge_shows_base));
    assert!(written, "it is remembered");
    ask(&mut state, Intent::ClosePage);
    assert_eq!(state.merge_pages.pages, []);
    open(&mut state);
    let (_, tree) = Window::new(ThemeName::Dark, MERGE).draw(&state, &[]);
    assert!(says(&tree, "Hide the base"));
    assert!(shown(&state).layout.rows.iter().any(|row| row.base.is_some()));
}

#[test]
fn merge_view_navigate__with_3_conflicts_it_reads_3_conflicts_left_and_after_accept_current_on_one_2() {
    let repository = conflicts_at(&[5, 15, 25]);
    let mut state = opened(&repository);
    let mut window = Window::new(ThemeName::Dark, MERGE);
    assert!(says(&window.draw(&state, &[]).1, "3 conflicts left"));
    let place = first_conflict(&state);
    page_intent(&mut state, MergePageIntent::Take(place, Choice::Current));
    assert!(says(&window.draw(&state, &[]).1, "2 conflicts left"));
}

#[test]
fn merge_view_navigate__after_the_last_conflict_is_resolved_it_reads_no_conflicts_left() {
    let repository = conflicts_at(&[10]);
    let mut state = opened(&repository);
    let place = first_conflict(&state);
    page_intent(&mut state, MergePageIntent::Take(place, Choice::Incoming));
    let (_, tree) = Window::new(ThemeName::Dark, MERGE).draw(&state, &[]);
    assert!(says(&tree, "No conflicts left"));
}

#[test]
fn merge_view_navigate__f7_on_the_last_region_goes_round_to_the_first() {
    let repository = conflicts_at(&[5, 15, 25]);
    let mut state = opened(&repository);
    let mut window = Window::new(ThemeName::Dark, MERGE);
    let _ = press(&mut window, &mut state, &[f7(Modifiers::NONE)]);
    let _ = press(&mut window, &mut state, &[f7(Modifiers::NONE)]);
    let tree = press(&mut window, &mut state, &[f7(Modifiers::NONE)]);
    assert!(says(&tree, "3 / 3"));
    let tree = press(&mut window, &mut state, &[f7(Modifiers::NONE)]);
    assert!(says(&tree, "1 / 3"));
    assert_eq!(shown(&state).current, Some(0));
    let tree = press(&mut window, &mut state, &[f7(Modifiers::SHIFT)]);
    assert!(says(&tree, "3 / 3"), "Shift+F7 before the first goes to the last");
}

#[test]
fn merge_view_page__a_change_of_each_side_and_a_conflict_are_drawn_in_their_colors() {
    let repository = stopped_merge(
        &[(4, "line 4 on main"), (12, "line 12 on main")],
        &[(12, "line 12 on feature"), (20, "line 20 on feature")],
    );
    let state = opened(&repository);
    let mut window = Window::new(ThemeName::Dark, MERGE);
    let _ = window.draw(&state, &[]);
    window.assert_snapshot("merge-page-dark");
}
