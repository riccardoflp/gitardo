//! Where the sections of the side column go, for every way they are folded (`repo.side-column`).

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use ui::Rect;

use super::side_column::{CHANGES_START, SectionHeights, SectionRects, SideSections, section_rects};
use super::side_sections::{SECTION_LEAST, SectionAsk, SectionPlace, section_places};

/// The height of a heading in the tests.
const HEADING: f32 = 28.0;
/// The height of a line in the tests.
const LINE: f32 = 1.0;
/// The column of the tests: 240 by 600, from 10 across and 50 down.
const COLUMN: Rect = Rect::new(10.0, 50.0, 240.0, 600.0);

/// Three sections, each folded or not, wanting 160, 280 and what is left.
fn three(folded: [bool; 3]) -> [SectionPlace; 3] {
    let wanted = [160.0, 280.0, SECTION_LEAST];
    let mut index = 0;
    let asks = folded.map(|is_folded| {
        let ask = SectionAsk {
            is_folded,
            wanted: wanted.get(index).copied().unwrap_or_default(),
        };
        index += 1;
        ask
    });
    section_places(COLUMN, asks, (HEADING, LINE))
}

/// Every way three sections are folded.
fn every_fold() -> Vec<[bool; 3]> {
    (0..8_u8)
        .map(|bits| [bits & 1 != 0, bits & 2 != 0, bits & 4 != 0])
        .collect()
}

#[test]
fn repo_side_column__every_heading_but_the_first_has_a_line_above_it_whatever_is_folded() {
    for folded in every_fold() {
        let places = three(folded);
        assert_eq!(places[0].line, None, "{folded:?}");
        for place in &places[1..] {
            let line = place
                .line
                .unwrap_or_else(|| panic!("a line above each heading: {folded:?}"));
            assert!((line.height - LINE).abs() < f32::EPSILON, "{folded:?}: {line:?}");
            assert!((line.width - COLUMN.width).abs() < f32::EPSILON, "{folded:?}: {line:?}");
            assert!(
                (line.bottom() - place.heading.y).abs() < f32::EPSILON,
                "the line touches its heading: {folded:?} {line:?} {:?}",
                place.heading
            );
        }
    }
}

#[test]
fn repo_side_column__the_parts_follow_one_another_from_the_top_and_never_overlap_whatever_is_folded() {
    for folded in every_fold() {
        let mut next = COLUMN.y;
        for (place, is_folded) in three(folded).iter().zip(folded) {
            if let Some(line) = place.line {
                assert!((line.y - next).abs() < 0.01, "{folded:?}: {line:?} at {next}");
                next = line.bottom();
            }
            assert!((place.heading.y - next).abs() < 0.01, "{folded:?}: {place:?} at {next}");
            assert!((place.heading.height - HEADING).abs() < 0.01, "{folded:?}: {place:?}");
            next = place.heading.bottom();
            assert_eq!(place.body.is_none(), is_folded, "{folded:?}: {place:?}");
            if let Some(body) = place.body {
                assert!((body.y - next).abs() < 0.01, "{folded:?}: {body:?} at {next}");
                next = body.bottom();
            }
        }
        assert!(next <= COLUMN.bottom() + 0.01, "{folded:?}: ends at {next}");
        if folded.contains(&false) {
            assert!(
                (next - COLUMN.bottom()).abs() < 0.01,
                "an open section fills the column: {folded:?} ends at {next}"
            );
        }
    }
}

#[test]
fn repo_side_column__each_open_section_but_the_last_has_its_height_and_the_last_takes_the_rest() {
    let [first, second, third] = three([false, false, false]);
    let height = |place: &SectionPlace| place.heading.height + place.body.map_or(0.0, |body| body.height);
    assert!((height(&first) - 160.0).abs() < 0.01, "{first:?}");
    assert!((height(&second) - 280.0).abs() < 0.01, "{second:?}");
    assert!(
        (height(&third) - (600.0 - 2.0 * LINE - 160.0 - 280.0)).abs() < 0.01,
        "{third:?}"
    );
    // With the last folded, the second is the last open one and takes what is left; the heading of the third is at
    // the foot of the column.
    let [first, second, third] = three([false, false, true]);
    assert!((height(&first) - 160.0).abs() < 0.01, "{first:?}");
    assert!(
        (height(&second) - (600.0 - 2.0 * LINE - HEADING - 160.0)).abs() < 0.01,
        "{second:?}"
    );
    assert!((third.heading.bottom() - COLUMN.bottom()).abs() < 0.01, "{third:?}");
    // With every section folded the headings are at the top, and the rest of the column is empty.
    let [_, _, third] = three([true, true, true]);
    assert!(
        (third.heading.bottom() - (COLUMN.y + 3.0 * HEADING + 2.0 * LINE)).abs() < 0.01,
        "{third:?}"
    );
}

#[test]
fn repo_side_column__only_the_line_under_an_open_section_with_an_open_one_under_it_is_dragged() {
    for folded in every_fold() {
        let places = three(folded);
        for (index, place) in places.iter().enumerate() {
            let is_open_after = folded.iter().skip(index + 1).any(|is_folded| !is_folded);
            let is_dragged = !folded[index] && is_open_after;
            assert_eq!(place.dragged.is_some(), is_dragged, "{folded:?}: section {index}");
            let Some(dragged) = place.dragged else {
                continue;
            };
            assert!((dragged.top - place.heading.y).abs() < 0.01, "{folded:?}: {dragged:?}");
            let body = place.body.map_or(0.0, |body| body.height);
            assert!(
                (dragged.height - (HEADING + body)).abs() < 0.01,
                "{folded:?}: {dragged:?}"
            );
        }
    }
    // With the middle section folded between two open ones, the line under the first is the one dragged.
    let [first, second, third] = three([false, true, false]);
    assert!(first.dragged.is_some() && second.dragged.is_none() && third.dragged.is_none());
}

#[test]
fn repo_side_column__a_section_is_never_shorter_than_its_least_while_there_is_room_nor_takes_it_from_those_under_it() {
    let tall = |wanted: f32| {
        let asks = [
            SectionAsk {
                is_folded: false,
                wanted,
            },
            SectionAsk {
                is_folded: false,
                wanted: 280.0,
            },
            SectionAsk {
                is_folded: false,
                wanted: SECTION_LEAST,
            },
        ];
        section_places(COLUMN, asks, (HEADING, LINE))
    };
    let height = |place: &SectionPlace| place.heading.height + place.body.map_or(0.0, |body| body.height);
    let [first, _, _] = tall(10.0);
    assert!((height(&first) - SECTION_LEAST).abs() < 0.01, "{first:?}");
    let [first, second, third] = tall(5_000.0);
    let room = 600.0 - 2.0 * LINE;
    assert!(
        (height(&first) - (room - 2.0 * SECTION_LEAST)).abs() < 0.01,
        "{first:?}"
    );
    assert!((height(&second) - SECTION_LEAST).abs() < 0.01, "{second:?}");
    assert!((height(&third) - SECTION_LEAST).abs() < 0.01, "{third:?}");
    let dragged = first
        .dragged
        .unwrap_or_else(|| panic!("the line under the first is dragged"));
    assert!((dragged.room - room).abs() < 0.01 && (dragged.least_after - 2.0 * SECTION_LEAST).abs() < 0.01);
}

/// The three sections of the side column as it lays them out, with rows of **Repositories** 26 high and what
/// **Changes** shows as high as given.
fn rects(sections: SideSections, changes: Option<f32>) -> SectionRects {
    let heights = SectionHeights {
        heading: HEADING,
        line: LINE,
        repositories: 26.0,
        changes,
    };
    section_rects(COLUMN, sections, &heights)
}

/// The height of a section, its heading included.
fn height_of(place: &SectionPlace) -> f32 {
    place.heading.height + place.body.map_or(0.0, |body| body.height)
}

#[test]
fn repo_side_column__repositories_and_changes_start_as_tall_as_what_they_show_or_at_160_and_280_and_branches_has_the_rest()
 {
    let short = rects(SideSections::default(), Some(100.0));
    assert!(
        (height_of(&short.repositories) - SECTION_LEAST).abs() < 0.01,
        "{short:?}"
    );
    assert!(
        (height_of(&short.changes) - (HEADING + 100.0)).abs() < 0.01,
        "{short:?}"
    );
    let rest = COLUMN.height - 2.0 * LINE - SECTION_LEAST - HEADING - 100.0;
    assert!((height_of(&short.branches) - rest).abs() < 0.01, "{short:?}");
    let tall = rects(SideSections::default(), Some(2_000.0));
    assert!((height_of(&tall.changes) - CHANGES_START).abs() < 0.01, "{tall:?}");
    let dragged = SideSections {
        repositories_height: Some(120.0),
        changes_height: Some(200.0),
        ..SideSections::default()
    };
    let places = rects(dragged, Some(2_000.0));
    assert!((height_of(&places.repositories) - 120.0).abs() < 0.01, "{places:?}");
    assert!((height_of(&places.changes) - 200.0).abs() < 0.01, "{places:?}");
    assert!(places.repositories.dragged.is_some() && places.changes.dragged.is_some());
    assert!(places.branches.dragged.is_none());
}

#[test]
fn repo_side_column__changes_is_its_heading_alone_while_no_repository_is_shown_and_its_line_is_not_dragged() {
    let places = rects(SideSections::default(), None);
    assert_eq!(places.changes.body, None);
    assert_eq!(places.changes.dragged, None);
    assert!(places.changes.line.is_some() && places.branches.line.is_some());
    assert!(
        (places.branches.heading.y - places.changes.heading.bottom() - LINE).abs() < 0.01,
        "{places:?}"
    );
    // The line under **Repositories** is still dragged, against **Branches**.
    assert!(places.repositories.dragged.is_some());
}

#[test]
fn repo_side_column__with_the_last_section_folded_the_one_above_it_takes_the_rest_whatever_it_was_dragged_to() {
    let folded = SideSections {
        changes_height: Some(90.0),
        is_branches_folded: true,
        ..SideSections::default()
    };
    let places = rects(folded, Some(2_000.0));
    let rest = COLUMN.height - 2.0 * LINE - HEADING - height_of(&places.repositories);
    assert!((height_of(&places.changes) - rest).abs() < 0.01, "{places:?}");
    assert_eq!(places.changes.dragged, None);
    assert!(
        (places.branches.heading.bottom() - COLUMN.bottom()).abs() < 0.01,
        "{places:?}"
    );
}
