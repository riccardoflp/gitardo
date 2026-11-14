//! The rules that resolve conflicts with no choice (`conflicts.resolve-by-rules`), run on the files in conflict as
//! they were read: how many conflicts each has, how many the rules resolve and why, and the text that results.

use model::conflicts::{ConflictTexts, RuleCounts};
use text::conflicts::{
    RuleCase, RuleResolution, RuleResolutions, RulesInput, parse_markers, resolve_by_rules, split_lines,
};

/// What the rules make of one file in conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileResolution {
    /// Its path from the top of the working tree.
    pub path: String,
    /// How many conflicts git marked in it.
    pub conflicts: usize,
    /// How many of them the rules resolve.
    pub resolved: usize,
    /// Why they could be resolved, each case once, in the order they come in the file.
    pub cases: Vec<RuleCase>,
    /// The file once its resolved conflicts are written, when any is; the others keep git's markers.
    pub text: Option<Vec<u8>>,
}

impl FileResolution {
    /// Whether every conflict of the file is resolved, so that it can be staged.
    #[must_use]
    pub fn is_resolved_in_full(&self) -> bool {
        self.conflicts > 0 && self.resolved == self.conflicts
    }
}

/// What the rules make of each file in conflict, in order. A file that is not there, that one side deleted, or that
/// the rules leave alone, resolves nothing; the conflicts git marked in it are still counted when it is text.
#[must_use]
pub fn resolve_files(texts: &[ConflictTexts]) -> Vec<FileResolution> {
    texts.iter().map(resolve_file).collect()
}

/// How many conflicts the files have, and how many of them the rules resolve.
#[must_use]
pub fn count_by_rules(texts: &[ConflictTexts]) -> RuleCounts {
    resolve_files(texts)
        .iter()
        .fold(RuleCounts::default(), |counts, file| RuleCounts {
            conflicts: counts.conflicts + file.conflicts,
            resolvable: counts.resolvable + file.resolved,
        })
}

/// What the rules make of one file.
fn resolve_file(texts: &ConflictTexts) -> FileResolution {
    let mut resolution = FileResolution {
        path: texts.path.clone(),
        conflicts: 0,
        resolved: 0,
        cases: Vec::new(),
        text: None,
    };
    let Some(file) = texts.file.as_deref() else {
        return resolution;
    };
    resolution.conflicts = std::str::from_utf8(file)
        .ok()
        .and_then(parse_markers)
        .map_or(0, |conflicts| conflicts.len());
    let (Some(current), Some(incoming)) = (texts.current.as_deref(), texts.incoming.as_deref()) else {
        return resolution;
    };
    let input = RulesInput {
        file,
        base: texts.base.as_deref().unwrap_or_default(),
        current,
        incoming,
        // The features read no clock: the rules stop on their own limits of size and of lines.
        keep_going: &|| true,
    };
    let RuleResolutions::LookedInto(conflicts) = resolve_by_rules(&input) else {
        return resolution;
    };
    let resolutions: Vec<_> = conflicts
        .iter()
        .filter_map(|conflict| conflict.resolution.as_ref())
        .collect();
    resolution.resolved = resolutions.len();
    for found in &resolutions {
        if !resolution.cases.contains(&found.case) {
            resolution.cases.push(found.case);
        }
    }
    if resolutions.is_empty() {
        return resolution;
    }
    resolution.text = std::str::from_utf8(file)
        .ok()
        .map(|text| rewritten(text, &resolutions).into_bytes());
    resolution
}

/// The text of a file with the lines of each conflict resolved replaced by those of its resolution.
fn rewritten(text: &str, resolutions: &[&RuleResolution<'_>]) -> String {
    let lines = split_lines(text);
    let mut written = String::with_capacity(text.len());
    let mut next = 0;
    for found in resolutions {
        let kept = lines.get(next..found.replaced.start).unwrap_or_default();
        for line in kept.iter().chain(&found.lines) {
            written.push_str(line.text);
            written.push_str(line.ending);
        }
        next = found.replaced.end;
    }
    for line in lines.get(next..).unwrap_or_default() {
        written.push_str(line.text);
        written.push_str(line.ending);
    }
    written
}

/// The words of a case, as the question and the review name it.
#[must_use]
pub fn case_words(case: RuleCase) -> &'static str {
    match case {
        RuleCase::SpacesOrLineEndings => "spaces or line endings only",
        RuleCase::SameChange => "the same change on both sides",
        RuleCase::OneSide => "a change on one side only",
        RuleCase::NextToEachOther => "changes to lines next to each other",
    }
}

#[cfg(test)]
mod tests {
    use super::{count_by_rules, resolve_files};
    use model::conflicts::ConflictTexts;

    fn texts(file: &str, sides: [&str; 3]) -> ConflictTexts {
        let [base, current, incoming] = sides.map(|side| Some(side.as_bytes().to_vec()));
        ConflictTexts {
            path: String::from("a.txt"),
            file: Some(file.as_bytes().to_vec()),
            base,
            current,
            incoming,
        }
    }

    #[test]
    fn writes_the_lines_next_to_each_other_and_keeps_what_is_around() {
        let file = "a\n<<<<<<< HEAD\nB\nc\n=======\nb\nC\n>>>>>>> feature\nd\n";
        let found = resolve_files(&[texts(file, ["a\nb\nc\nd\n", "a\nB\nc\nd\n", "a\nb\nC\nd\n"])]);
        let first = found.first().unwrap();
        assert_eq!((first.conflicts, first.resolved), (1, 1));
        assert_eq!(first.text.as_deref(), Some("a\nB\nC\nd\n".as_bytes()));
        assert!(first.is_resolved_in_full());
    }

    #[test]
    fn counts_a_conflict_that_needs_a_choice_and_writes_nothing_for_it() {
        let file = "a\n<<<<<<< HEAD\nB1\n=======\nB2\n>>>>>>> feature\nc\n";
        let one = [texts(file, ["a\nb\nc\n", "a\nB1\nc\n", "a\nB2\nc\n"])];
        let counts = count_by_rules(&one);
        assert_eq!((counts.conflicts, counts.resolvable), (1, 0));
        assert_eq!(resolve_files(&one).first().unwrap().text, None);
    }
}
