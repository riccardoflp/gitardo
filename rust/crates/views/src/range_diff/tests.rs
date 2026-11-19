use model::CommitId;
use model::range_diff::{Line, Pair, PairKind, RangeDiff, RangeDiffTarget, Ranges};
use ui::{Key, KeyEvent, Modifiers, NamedKey};

use super::{DiffTone, RangeDiffIntent, RangeDiffRead, RewriteState, diff_lines, keyed, pair_row};

/// A commit named by a digit.
fn commit(digit: char) -> CommitId {
    CommitId::parse(&digit.to_string().repeat(40)).unwrap()
}

/// A pair with its commits by digit, `-` for none.
fn pair(kind: PairKind, (before, now): (char, char), subject: &str, diff: &[&str]) -> Pair {
    let side = |digit: char| (digit != '-').then(|| commit(digit));
    Pair {
        kind,
        before: side(before),
        now: side(now),
        subject: subject.to_owned(),
        diff: diff.iter().map(|line| (*line).to_owned()).collect(),
    }
}

/// After a rebase in which `B` was reworded and `D` dropped.
fn rewritten() -> RangeDiff {
    RangeDiff {
        pairs: vec![
            pair(PairKind::Same, ('1', '5'), "Add A", &[]),
            pair(
                PairKind::Changed,
                ('2', '6'),
                "Add B, better",
                &[
                    "@@ Metadata",
                    " Author: Ada <ada@example.com>",
                    "  ## Commit message ##",
                    "-    Add B",
                    "+    Add B, better",
                    "  ## b.txt (new) ##",
                ],
            ),
            pair(PairKind::Same, ('3', '7'), "Add C", &[]),
            pair(PairKind::Dropped, ('4', '-'), "Add D", &[]),
        ],
        before_count: 4,
        now_count: 3,
    }
}

/// The page of `feature` with its pairs read, nothing opened.
fn state<'a>(target: &'a RangeDiffTarget, read: &'a RangeDiff, lit: Option<usize>) -> RewriteState<'a> {
    RewriteState {
        target,
        title: "feature \u{b7} what changed",
        read: RangeDiffRead::Read(read),
        opened: &[],
        lit,
    }
}

/// The target of the page of `feature`.
fn target() -> RangeDiffTarget {
    let line = |to: &str| Line {
        from: String::from("main"),
        to: to.to_owned(),
    };
    RangeDiffTarget {
        title: String::from("What the rebase of feature changed"),
        branch: String::from("feature"),
        ranges: Ranges {
            arguments: Vec::new(),
            before: line("old"),
            now: line("feature"),
        },
    }
}

#[test]
fn range_diff_page__after_a_rebase_with_b_reworded_and_d_dropped_it_lists_a_dimmed_b_changed_c_and_d_dropped() {
    let read = rewritten();
    let rows: Vec<_> = read.pairs.iter().map(pair_row).collect();
    assert_eq!(
        rows.iter().map(|row| (row.kind, row.is_dimmed)).collect::<Vec<_>>(),
        [
            (PairKind::Same, true),
            (PairKind::Changed, false),
            (PairKind::Same, true),
            (PairKind::Dropped, false)
        ]
    );
    assert_eq!(rows[1].changed.as_deref(), Some("message"));
    assert_eq!(read.counts(), "2 the same \u{b7} 1 changed \u{b7} 1 dropped");
}

#[test]
fn range_diff_page__opening_the_row_of_b_shows_the_old_subject_lost_and_the_new_one_gained() {
    let (target, read) = (target(), rewritten());
    let enter = KeyEvent::press(Key::Named(NamedKey::Enter), Modifiers::NONE);
    assert_eq!(
        keyed(&state(&target, &read, Some(1)), enter),
        Some(RangeDiffIntent::Toggle(1))
    );
    let lines = diff_lines(&read.pairs[1]);
    assert!(lines.contains(&(DiffTone::Heading, " ## Commit message ##")));
    assert!(lines.contains(&(DiffTone::Lost, "    Add B")));
    assert!(lines.contains(&(DiffTone::Gained, "    Add B, better")));
}

#[test]
fn range_diff_page__the_arrows_move_through_the_rows() {
    let (target, read) = (target(), rewritten());
    let key = |named| KeyEvent::press(Key::Named(named), Modifiers::NONE);
    assert_eq!(
        keyed(&state(&target, &read, None), key(NamedKey::ArrowDown)),
        Some(RangeDiffIntent::Light(0))
    );
    assert_eq!(keyed(&state(&target, &read, Some(3)), key(NamedKey::ArrowDown)), None);
    assert_eq!(
        keyed(&state(&target, &read, Some(2)), key(NamedKey::ArrowUp)),
        Some(RangeDiffIntent::Light(1))
    );
}

#[test]
fn range_diff_page__a_commit_added_by_the_rewrite_has_a_row_with_no_hash_before() {
    let added = pair(PairKind::Added, ('-', '8'), "Add the tests", &[]);
    let row = pair_row(&added);
    assert_eq!((row.before, row.now), (None, Some("8888888")));
    assert!(!row.is_dimmed);
}
