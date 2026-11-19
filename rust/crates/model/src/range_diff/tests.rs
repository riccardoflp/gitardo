use super::{Pair, PairKind, RangeDiff, Ranges, RangesInput, Rewrite, RewriteKind, Side, ranges};
use crate::CommitId;

fn commit(digit: char) -> CommitId {
    CommitId::parse(&digit.to_string().repeat(40)).unwrap()
}

fn arguments(found: Option<Ranges>) -> Vec<String> {
    found.map(|found| found.arguments).unwrap_or_default()
}

#[test]
fn range_diff_ranges__a_rebase_of_feature_from_a1_to_b2_onto_m3_gives_the_form_of_three() {
    let rewrite = Rewrite {
        kind: RewriteKind::Rebase { onto: commit('3') },
        before: commit('1'),
        after: commit('2'),
    };
    let found = ranges(&RangesInput::Rewrite(&rewrite)).unwrap();
    assert_eq!(
        found.arguments,
        [commit('3'), commit('1'), commit('2')].map(|commit| commit.to_string())
    );
    assert_eq!(found.before.range(), format!("{}..{}", commit('3'), commit('1')));
    assert_eq!(found.now.range(), format!("{}..{}", commit('3'), commit('2')));
}

#[test]
fn range_diff_ranges__an_amend_of_c4_into_c5_gives_each_with_its_parent() {
    let rewrite = Rewrite {
        kind: RewriteKind::Amend { is_root: false },
        before: commit('4'),
        after: commit('5'),
    };
    assert_eq!(
        arguments(ranges(&RangesInput::Rewrite(&rewrite))),
        [
            format!("{}^..{}", commit('4'), commit('4')),
            format!("{}^..{}", commit('5'), commit('5'))
        ]
    );
    let root = Rewrite {
        kind: RewriteKind::Amend { is_root: true },
        ..rewrite
    };
    assert_eq!(
        ranges(&RangesInput::Rewrite(&root)),
        None,
        "the first commit has nothing before it"
    );
}

#[test]
fn range_diff_ranges__feature_rebased_after_it_was_pushed_gives_the_ranges_of_the_rebase_from_its_upstream() {
    let rebase = Rewrite {
        kind: RewriteKind::Rebase { onto: commit('3') },
        before: commit('1'),
        after: commit('2'),
    };
    let input = RangesInput::Upstream {
        branch: Side {
            name: "feature",
            commit: &commit('2'),
        },
        upstream: Side {
            name: "origin/feature",
            commit: &commit('1'),
        },
        last_rewrite: Some(&rebase),
    };
    assert_eq!(
        arguments(ranges(&input)),
        [
            commit('3').to_string(),
            String::from("origin/feature"),
            String::from("feature")
        ]
    );
}

#[test]
fn range_diff_ranges__a_branch_and_its_upstream_that_diverged_with_no_rebase_give_the_symmetric_form() {
    let input = RangesInput::Upstream {
        branch: Side {
            name: "feature",
            commit: &commit('2'),
        },
        upstream: Side {
            name: "origin/feature",
            commit: &commit('1'),
        },
        last_rewrite: None,
    };
    let found = ranges(&input).unwrap();
    assert_eq!(found.arguments, ["origin/feature...feature"]);
    assert_eq!(found.before.range(), "feature..origin/feature");
    assert_eq!(found.now.range(), "origin/feature..feature");
    let same = RangesInput::Upstream {
        branch: Side {
            name: "feature",
            commit: &commit('1'),
        },
        upstream: Side {
            name: "origin/feature",
            commit: &commit('1'),
        },
        last_rewrite: None,
    };
    assert_eq!(ranges(&same), None, "two sides at the same commit give nothing");
}

#[test]
fn range_diff_ranges__a_remote_branch_force_pushed_gives_before_and_after() {
    let pushed = Rewrite {
        kind: RewriteKind::ForcePush,
        before: commit('1'),
        after: commit('2'),
    };
    assert_eq!(
        arguments(ranges(&RangesInput::Rewrite(&pushed))),
        [format!("{}...{}", commit('1'), commit('2'))]
    );
}

#[test]
fn range_diff_page__the_counts_leave_out_the_kinds_with_none() {
    let pair = |kind| Pair {
        kind,
        before: None,
        now: None,
        subject: String::new(),
        diff: [
            "@@ Metadata",
            " Author: Ada <ada@example.com>",
            "  ## Commit message ##",
            "-    Old words",
            "+    New words",
            "  ## parser.rs (new) ##",
            " @@",
        ]
        .map(String::from)
        .to_vec(),
    };
    let compared = RangeDiff {
        pairs: vec![
            pair(PairKind::Same),
            pair(PairKind::Changed),
            pair(PairKind::Same),
            pair(PairKind::Dropped),
        ],
        before_count: 4,
        now_count: 3,
    };
    assert_eq!(compared.counts(), "2 the same \u{b7} 1 changed \u{b7} 1 dropped");
    let parts = compared.pairs[1].changed_parts();
    assert!(parts.message && !parts.changes);
}
