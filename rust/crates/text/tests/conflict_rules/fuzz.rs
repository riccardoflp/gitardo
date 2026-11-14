//! Files made at random, merged by git and resolved by the rules, with the promises of the card checked on each: a few
//! hundred in every `cargo test`, thousands in the test left out by default
//! (`cargo test -p text --test conflict_rules -- --ignored`). Each test says its seed when it fails;
//! `GITARDO_TEST_SEED=<seed>` runs it again with the same files.

use testkit::Random;
use text::conflicts::{RuleConflict, RuleResolutions, RulesInput, resolve_by_rules, split_lines};

use crate::check::{as_written, assert_left_as_git_wrote, assert_lines_of_the_sides, rules, with_resolutions};
use crate::merges::{Case, Style, merged, merged_in_styles};
use crate::numbered::{Collisions, Numbered, without_spacers};
use crate::repeating::repeating;

/// How many conflicts the rules saw, and how many they resolved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Counts {
    /// The conflicts git marked.
    conflicts: usize,
    /// Those the rules resolved.
    resolved: usize,
}

/// Checks the rules on files of numbered lines merged by git in some styles: a conflict resolved gives the merge
/// expected over its stretch, and one where the sides collide is left; a file git merged without conflict, or whose
/// conflicts are all resolved, is the merge expected; every line written is a line of a side; what is left is left
/// as git wrote it.
fn numbered_files(random: &mut Random, files: usize, collisions: Collisions, styles: &[Style]) -> Counts {
    let made: Vec<Numbered> = (0..files).map(|_| Numbered::new(random, collisions)).collect();
    let cases: Vec<Case> = made.iter().map(Numbered::case).collect();
    let mut counts = Counts::default();
    for (style, files) in styles.iter().zip(merged_in_styles(&cases, styles)) {
        for ((numbered, case), file) in made.iter().zip(&cases).zip(&files) {
            if file.contains("<<<<<<<") {
                check_numbered(numbered, case, file, &mut counts);
            } else {
                assert_eq!(
                    Some(file.clone()),
                    numbered.expected_file(),
                    "git merged {case:?} ({style:?})"
                );
            }
        }
    }
    counts
}

/// Checks the rules on one file of numbered lines.
fn check_numbered(numbered: &Numbered, case: &Case, file: &str, counts: &mut Counts) {
    let conflicts = rules(file, case);
    assert_lines_of_the_sides(file, &conflicts);
    assert_left_as_git_wrote(file, &conflicts);
    for conflict in &conflicts {
        counts.conflicts += 1;
        let Some(resolution) = &conflict.resolution else {
            continue;
        };
        counts.resolved += 1;
        let stretch = conflict.stretch.as_ref().expect("a conflict resolved has a stretch");
        let written: Vec<String> = resolution.lines.iter().map(|line| line.text.to_owned()).collect();
        assert_eq!(
            Some(written),
            numbered.expected(stretch.base_lines.start, stretch.base_lines.end),
            "the conflict at line {} of {case:?}:\n{file}",
            conflict.conflict.line
        );
    }
    if conflicts.iter().all(|conflict| conflict.resolution.is_some()) {
        assert_eq!(
            Some(with_resolutions(file, &conflicts)),
            numbered.expected_file(),
            "{case:?}:\n{file}"
        );
    }
}

/// Checks that the merge expected of files of numbered lines is what git writes when it merges them without conflict:
/// their spread versions, where changes to lines next to each other are apart. Returns how many git merged so.
fn expected_is_what_git_merges(made: &[Numbered]) -> usize {
    let spread: Vec<Case> = made.iter().map(Numbered::spread).collect();
    let mut compared = 0;
    for (numbered, file) in made.iter().zip(merged(&spread, Style::Merge)) {
        if !file.contains("<<<<<<<") {
            assert_eq!(Some(without_spacers(&file)), numbered.expected_file(), "{numbered:?}");
            compared += 1;
        }
    }
    compared
}

/// Checks the rules on files of lines that repeat merged by git in some styles: every line written is a line of a
/// side, what is left is left as git wrote it, and in the `diff3` style a conflict resolved from the whole file is
/// resolved to the same lines from the three parts git wrote between its markers alone.
fn repeating_files(random: &mut Random, files: usize, styles: &[Style]) -> Counts {
    let cases: Vec<Case> = (0..files).map(|_| repeating(random)).collect();
    let mut counts = Counts::default();
    for (style, files) in styles.iter().zip(merged_in_styles(&cases, styles)) {
        for (case, file) in cases.iter().zip(&files).filter(|(_, file)| file.contains("<<<<<<<")) {
            let conflicts = rules(file, case);
            assert_lines_of_the_sides(file, &conflicts);
            assert_left_as_git_wrote(file, &conflicts);
            if *style == Style::Diff3 {
                assert_parts_alone_resolve_the_same(case, file, &conflicts);
            }
            counts.conflicts += conflicts.len();
            counts.resolved += conflicts
                .iter()
                .filter(|conflict| conflict.resolution.is_some())
                .count();
        }
    }
    counts
}

/// Checks that every conflict resolved from a whole file in the `diff3` style is also resolved, to the same lines,
/// when the rules are given only the three parts git wrote between its markers.
fn assert_parts_alone_resolve_the_same(case: &Case, file: &str, conflicts: &[RuleConflict<'_>]) {
    let lines = split_lines(file);
    let joined = |range: std::ops::Range<usize>| as_written(&lines[range]);
    for conflict in conflicts {
        let Some(resolution) = &conflict.resolution else {
            continue;
        };
        let marked = &conflict.conflict;
        let base = joined(marked.base.clone().expect("the diff3 style writes the base"));
        let (current, incoming) = (joined(marked.current.clone()), joined(marked.incoming.clone()));
        let alone = format!("<<<<<<< main\n{current}||||||| base\n{base}=======\n{incoming}>>>>>>> incoming\n");
        let always = || true;
        let found = resolve_by_rules(&RulesInput {
            file: alone.as_bytes(),
            base: base.as_bytes(),
            current: current.as_bytes(),
            incoming: incoming.as_bytes(),
            keep_going: &always,
        });
        let RuleResolutions::LookedInto(parts) = found else {
            panic!("the parts were left alone: {found:?}");
        };
        let part = parts[0].resolution.as_ref().unwrap_or_else(|| {
            panic!(
                "the conflict at line {} of {case:?} is resolved in the whole file but not from its parts:\n{file}",
                marked.line
            )
        });
        let whole: Vec<&str> = resolution.lines.iter().map(|line| line.text).collect();
        let from_parts: Vec<&str> = lines[resolution.replaced.start..marked.span.start]
            .iter()
            .chain(&part.lines)
            .chain(&lines[marked.span.end..resolution.replaced.end])
            .map(|line| line.text)
            .collect();
        assert_eq!(
            from_parts, whole,
            "the conflict at line {} of {case:?}:\n{file}",
            marked.line
        );
    }
}

#[test]
fn conflicts_resolve_by_rules__numbered_lines_merge_as_expected_in_every_style() {
    let mut random = Random::from_environment(120);
    let counts = numbered_files(&mut random, 120, Collisions::Never, &Style::ALL);
    assert!(counts.resolved * 2 > counts.conflicts, "{counts:?}");
    let made: Vec<Numbered> = (0..120)
        .map(|_| Numbered::new(&mut random, Collisions::Never))
        .collect();
    assert!(expected_is_what_git_merges(&made) > 0);
}

#[test]
fn conflicts_resolve_by_rules__every_line_written_is_a_line_of_a_side() {
    let mut random = Random::from_environment(2);
    let _ = numbered_files(&mut random, 100, Collisions::Sometimes, &[Style::Merge]);
    let _ = repeating_files(&mut random, 100, &[Style::Merge]);
}

#[test]
fn conflicts_resolve_by_rules__a_conflict_left_is_as_git_wrote_it() {
    let mut random = Random::from_environment(3);
    let counts = numbered_files(&mut random, 100, Collisions::Sometimes, &[Style::Zdiff3]);
    assert!(counts.resolved < counts.conflicts, "{counts:?}");
    let _ = repeating_files(&mut random, 100, &[Style::Zdiff3]);
}

#[test]
fn conflicts_resolve_by_rules__the_three_parts_alone_resolve_as_the_whole_file() {
    let mut random = Random::from_environment(150);
    let counts = repeating_files(&mut random, 150, &[Style::Diff3]);
    assert!(counts.resolved > 0, "{counts:?}");
}

#[test]
#[ignore = "thousands of files merged by git take minutes; run with --ignored"]
fn conflicts_resolve_by_rules__thousands_of_files_against_git() {
    let mut random = Random::from_environment(2026);
    let expected = numbered_files(&mut random, 1500, Collisions::Never, &Style::ALL);
    let colliding = numbered_files(&mut random, 1500, Collisions::Sometimes, &Style::ALL);
    let made: Vec<Numbered> = (0..1500)
        .map(|_| Numbered::new(&mut random, Collisions::Never))
        .collect();
    let compared = expected_is_what_git_merges(&made);
    let repeating = repeating_files(&mut random, 3000, &Style::ALL);
    println!(
        "numbered: {expected:?}; colliding: {colliding:?}; compared with git: {compared}; repeating: {repeating:?}"
    );
    assert!(expected.resolved * 2 > expected.conflicts, "{expected:?}");
}
