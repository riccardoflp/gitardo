//! The lines of the graph laid out a page at a time (`graph.lanes`, D54): over a real history of 10,000 commits, each
//! page from where the one before it stopped gives what the whole history laid out at once gives.

use std::collections::HashSet;

use app::work::pages::read_repository_at_once;
use git::CancelToken;
use model::graph::{DateTime, GraphInput, LaneState, Row, lanes, lanes_after};
use testkit::Repository;
use views::graph::{LayoutInput, TimelineLayout};

use super::histories::import;
use super::speed::{first_page, history_of};

/// The rows of a history of 10,000 commits, a merge of a topic branch every ten, with branches and tags, as the window
/// reads them, and the repository they are of.
fn ten_thousand() -> (Vec<Row>, model::state::RepositoryState) {
    let repository = Repository::new();
    import(&repository, &history_of(1_000));
    let (state, page) = read_repository_at_once(repository.path(), &first_page(), &CancelToken::new()).unwrap();
    assert_eq!(page.rows.len(), 10_000);
    (page.rows, state)
}

/// The cells of rows laid out in parts cut at some rows, each from the lanes under the part before.
fn in_parts(inputs: &[GraphInput<'_>], cuts: &[usize]) -> Vec<model::graph::GraphCell> {
    let mut cells = Vec::with_capacity(inputs.len());
    let mut state = LaneState::default();
    let mut start = 0;
    for &end in cuts.iter().chain(std::iter::once(&inputs.len())) {
        let (part, under) = lanes_after(&state, inputs.get(start..end).unwrap());
        cells.extend(part);
        state = under;
        start = end;
    }
    cells
}

#[test]
fn graph_lanes__10000_rows_of_a_real_history_laid_out_a_page_at_a_time_give_the_cells_of_all_at_once() {
    let (rows, _) = ten_thousand();
    let inputs: Vec<GraphInput<'_>> = rows.iter().map(Row::graph_input).collect();
    let all = lanes(&inputs);
    let pages: Vec<usize> = (1..10).map(|page| page * 1_000).collect();
    assert_eq!(in_parts(&inputs, &pages), all, "pages of 1,000");
    let merge = rows.iter().position(|row| row.parents().len() > 1).unwrap();
    assert_eq!(
        in_parts(&inputs, &[merge + 1]),
        all,
        "cut after the merge at row {merge}"
    );
    let before_parent = (500..rows.len())
        .find(|&index| {
            let next = rows.get(index + 1).and_then(Row::id);
            rows.get(index).and_then(|row| row.parents().first()) == next && next.is_some()
        })
        .unwrap();
    assert_eq!(
        in_parts(&inputs, &[before_parent + 1]),
        all,
        "cut between row {before_parent} and its first parent"
    );
}

/// The timeline of 10,000 commits laid out a page of 1,000 at a time, each from where the one before it stopped, and
/// laid out at once.
fn by_pages_and_at_once(one_lane: bool) -> (TimelineLayout, TimelineLayout) {
    let (rows, repository) = ten_thousand();
    let folded = HashSet::new();
    let input = |end: usize| LayoutInput {
        rows: rows.get(..end).unwrap(),
        repository: &repository,
        folded: &folded,
        combine_upstream: true,
        separators_at: Some(DateTime {
            moment: model::Timestamp(2_000_000_000),
            utc_offset: 0,
        }),
        one_lane,
    };
    let mut layout = TimelineLayout::new(&input(1_000));
    for end in (2..=10).map(|page| page * 1_000) {
        assert!(layout.extend(&input(end)), "the page ending at {end} is laid out alone");
    }
    (layout, TimelineLayout::new(&input(10_000)))
}

#[test]
fn graph_lanes__the_timeline_laid_out_a_page_at_a_time_is_the_one_laid_out_at_once() {
    let (by_pages, at_once) = by_pages_and_at_once(false);
    assert_eq!(by_pages, at_once);
}

#[test]
fn graph_one_lane__on_a_page_added_under_the_rows_is_on_the_one_line_as_all_rows_laid_out_at_once() {
    let (by_pages, at_once) = by_pages_and_at_once(true);
    assert_eq!(by_pages.lane_count, 1);
    assert!(
        by_pages.cells.iter().all(|cell| cell.lane == 0),
        "every dot in the first lane"
    );
    let joined = by_pages.cells.get(999..=1_000).unwrap();
    assert!(
        joined.iter().all(|cell| cell.lines.len() == 2),
        "the last row of the first page and the first of the second are joined: {joined:?}"
    );
    assert_eq!(by_pages, at_once);
}
