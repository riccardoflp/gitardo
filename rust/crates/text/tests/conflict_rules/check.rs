//! The rules run on a file git wrote, the resolutions put in, and the promises of the card checked on what they did.

use text::conflicts::{RuleConflict, RuleResolutions, RulesInput, parse_markers, resolve_by_rules, split_lines};

use crate::merges::Case;

/// Lines, each ended by a line feed.
pub fn ended<Text: AsRef<str>>(lines: &[Text]) -> String {
    lines.iter().fold(String::new(), |mut text, line| {
        text.push_str(line.as_ref());
        text.push('\n');
        text
    })
}

/// Lines with what ends each of them, as they are in the file.
pub fn as_written(lines: &[text::conflicts::Line<'_>]) -> String {
    lines.iter().fold(String::new(), |mut text, line| {
        text.push_str(line.text);
        text.push_str(line.ending);
        text
    })
}

/// The rules on a file git wrote for a case; panics when the file is left alone.
pub fn rules<'a>(file: &'a str, case: &'a Case) -> Vec<RuleConflict<'a>> {
    let always = || true;
    let found = resolve_by_rules(&RulesInput {
        file: file.as_bytes(),
        base: case.base.as_deref().unwrap_or_default().as_bytes(),
        current: case.current.as_bytes(),
        incoming: case.incoming.as_bytes(),
        keep_going: &always,
    });
    match found {
        RuleResolutions::LookedInto(conflicts) => conflicts,
        RuleResolutions::LeftAlone(why) => panic!("the file was left alone ({why:?}):\n{file}"),
    }
}

/// A file with the lines of each resolution put in place of those it replaces.
pub fn with_resolutions(file: &str, conflicts: &[RuleConflict<'_>]) -> String {
    let lines = split_lines(file);
    let mut text = String::new();
    let mut next = 0;
    let mut push = |line: &text::conflicts::Line<'_>| {
        text.push_str(line.text);
        text.push_str(line.ending);
    };
    for resolution in conflicts.iter().filter_map(|conflict| conflict.resolution.as_ref()) {
        lines[next..resolution.replaced.start].iter().for_each(&mut push);
        resolution.lines.iter().for_each(&mut push);
        next = resolution.replaced.end;
    }
    lines[next..].iter().for_each(&mut push);
    text
}

/// Checks that every line of every result is, without its ending, a line of the current or the incoming side of its
/// stretch.
pub fn assert_lines_of_the_sides(file: &str, conflicts: &[RuleConflict<'_>]) {
    for conflict in conflicts {
        let Some(resolution) = &conflict.resolution else {
            continue;
        };
        let stretch = conflict.stretch.as_ref().expect("a conflict resolved has a stretch");
        for line in &resolution.lines {
            assert!(
                stretch
                    .current
                    .iter()
                    .chain(&stretch.incoming)
                    .any(|side| side.text == line.text),
                "{:?} is in neither side of the conflict at line {}:\n{file}",
                line.text,
                conflict.conflict.line
            );
        }
    }
}

/// Checks that the conflicts left are byte for byte as git wrote them once the others are resolved, markers and the
/// names after them included, and that no resolution replaces a line of one of them.
pub fn assert_left_as_git_wrote(file: &str, conflicts: &[RuleConflict<'_>]) {
    let lines = split_lines(file);
    let span_text = |lines: &[text::conflicts::Line<'_>], span: std::ops::Range<usize>| as_written(&lines[span]);
    let left: Vec<String> = conflicts
        .iter()
        .filter(|conflict| conflict.resolution.is_none())
        .map(|conflict| span_text(&lines, conflict.conflict.span.clone()))
        .collect();
    for resolution in conflicts.iter().filter_map(|conflict| conflict.resolution.as_ref()) {
        for conflict in conflicts.iter().filter(|conflict| conflict.resolution.is_none()) {
            let span = &conflict.conflict.span;
            assert!(
                resolution.replaced.end <= span.start || span.end <= resolution.replaced.start,
                "a resolution replaces lines of the conflict left at line {}:\n{file}",
                conflict.conflict.line
            );
        }
    }
    let resolved = with_resolutions(file, conflicts);
    let resolved_lines = split_lines(&resolved);
    let found: Vec<String> = parse_markers(&resolved)
        .expect("the markers left pair up")
        .into_iter()
        .map(|conflict| span_text(&resolved_lines, conflict.span))
        .collect();
    assert_eq!(found, left, "the conflicts left are not as git wrote them:\n{file}");
}
