//! What the banner of a bisect says, and how the marks of the commits look.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::CommitId;
use model::bisect::Bisect;
use model::graph::BisectMark;

use super::{BannerSays, BisectBanner, marks_view};

fn commit(digit: char) -> CommitId {
    CommitId::parse(&digit.to_string().repeat(40)).unwrap_or_else(|| panic!("no hash"))
}

fn says(bisect: &Bisect, head: char) -> BannerSays {
    let head = commit(head);
    BisectBanner {
        bisect,
        head: Some(&head),
        is_running: false,
    }
    .says()
}

#[test]
fn bisect_banner__with_a_good_and_a_bad_and_c_checked_out_it_says_to_test_c() {
    let bisect = Bisect {
        bad: Some(commit('e')),
        good: vec![commit('a')],
        skipped: Vec::new(),
        remaining: Some(4),
    };
    assert_eq!(
        says(&bisect, 'c'),
        BannerSays::Test(String::from(
            "Bisect: test ccccccc \u{b7} 4 commits can still be the first bad one"
        ))
    );
    let only_bad = Bisect {
        good: Vec::new(),
        remaining: None,
        ..bisect.clone()
    };
    assert_eq!(says(&only_bad, 'e'), BannerSays::MarkGood);
    assert_eq!(says(&Bisect::default(), 'e'), BannerSays::MarkBad);
    let found = Bisect {
        remaining: Some(1),
        ..bisect
    };
    assert_eq!(says(&found, 'e'), BannerSays::Found(commit('e')));
}

#[test]
fn bisect_marks__each_mark_has_its_own_icon() {
    let icons: Vec<_> = [BisectMark::Good, BisectMark::Bad, BisectMark::Skipped]
        .into_iter()
        .map(|mark| marks_view(mark).0)
        .collect();
    assert_eq!(icons.len(), 3);
    assert_ne!(icons.first(), icons.get(1));
    assert_ne!(icons.get(1), icons.get(2));
}
