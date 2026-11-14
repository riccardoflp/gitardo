//! `file-history.lines`: the timeline narrowed to the commits that changed some lines of a file, followed back from the
//! commit checked out (`git log -L`).

use model::contract::{AnswerKey, Answers, Context, Outcome, Query, Refusal, Target};
use model::file_history::{CommittedLines, CommittedLinesQuery, HistoryOf, LineRange};

use super::{file_name, path_in_repository, shows};

/// The key of where the lines are in the commit checked out.
pub const COMMITTED: AnswerKey = AnswerKey("committed lines");

/// Said with no File page of a file of the repository.
const OPEN_A_FILE: &str = "Open a file of this repository to see the history of its lines.";
/// Said when the one line selected was added since the last commit.
const LINE_ADDED: &str = "This line was added after the last commit, and has no history yet.";
/// Said when every line selected was added since the last commit.
const LINES_ADDED: &str = "These lines were added after the last commit, and have no history yet.";

/// Finds the lines selected in the File page, or the line of the cursor, in the commit checked out
/// (`file-history.read-committed-lines`), then narrows the timeline to the commits that changed them, followed back
/// from that commit whatever branches the timeline shows. A file renamed in the index is looked for under its old name.
///
/// Feature: file-history.lines
#[must_use]
pub fn lines(context: &Context, answers: &Answers) -> Outcome {
    let Target::Lines(selected) = &context.target else {
        return Outcome::Refuse(Refusal::saying(OPEN_A_FILE));
    };
    let Some(path) = path_in_repository(context, &selected.file).filter(|path| !path.is_empty()) else {
        return Outcome::Refuse(Refusal::saying(OPEN_A_FILE));
    };
    let Some(committed) = answers.committed_lines(COMMITTED) else {
        let renamed_from = context
            .repository
            .working_tree
            .staged
            .iter()
            .find(|change| change.path == path)
            .and_then(|change| change.original_path.clone());
        let query = CommittedLinesQuery {
            path,
            renamed_from,
            text: selected.text.clone(),
            lines: selected.lines,
        };
        return Outcome::Read(COMMITTED, Query::CommittedLines(Box::new(query)));
    };
    let (commit, committed_path, range) = match committed {
        CommittedLines::InNoCommit => {
            let said = format!(
                "{} is in no commit yet, so its lines have no history.",
                file_name(&path)
            );
            return Outcome::Refuse(Refusal::saying(&said));
        }
        CommittedLines::Committed { lines: None, .. } if selected.lines.start == selected.lines.end => {
            return Outcome::Refuse(Refusal::saying(LINE_ADDED));
        }
        CommittedLines::Committed { lines: None, .. } => return Outcome::Refuse(Refusal::saying(LINES_ADDED)),
        CommittedLines::Committed {
            commit,
            path,
            lines: Some(range),
        } => (commit, path, range),
    };
    let title = format!("Show the history of {} of {committed_path}", lines_words(*range));
    let history = HistoryOf::Lines {
        path: committed_path.clone(),
        lines: *range,
        commit: commit.clone(),
        in_editor: (*range != selected.lines).then_some(selected.lines),
    };
    Outcome::Run(shows(title, Some(history)))
}

/// Some lines in words: `line 4`, `lines 4–9`.
#[must_use]
pub fn lines_words(range: LineRange) -> String {
    if range.start == range.end {
        format!("line {}", range.start)
    } else {
        format!("lines {}–{}", range.start, range.end)
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answer, Answers, Outcome, Query, Target};
    use model::file_history::{CommittedLines, HistoryOf, LineRange, LinesOfFile};

    use super::{COMMITTED, lines};
    use crate::file_history::testing::{on_disk, shown};
    use crate::testing::{commit, context, renamed, state};

    /// Lines of `src/app.rs` selected in its File page.
    fn selected(start: usize, end: usize) -> Target {
        Target::Lines(Box::new(LinesOfFile {
            file: on_disk("src/app.rs"),
            text: String::from("a\nb\n"),
            lines: LineRange { start, end },
        }))
    }

    /// The answers with where the lines are in `commit('a')`.
    fn found(range: Option<(usize, usize)>) -> Answers {
        let committed = CommittedLines::Committed {
            commit: commit('a'),
            path: String::from("src/app.rs"),
            lines: range.map(|(start, end)| LineRange { start, end }),
        };
        Answers::new().with(COMMITTED, Answer::CommittedLines(committed))
    }

    #[test]
    fn file_history_lines__follows_the_lines_of_the_commit_checked_out() {
        let context = context(state(), selected(4, 9));
        let Outcome::Read(COMMITTED, Query::CommittedLines(query)) = lines(&context, &Answers::new()) else {
            panic!("reads where the lines are");
        };
        assert_eq!(
            (query.path.as_str(), query.lines),
            ("src/app.rs", LineRange { start: 4, end: 9 })
        );
        assert_eq!(
            shown(lines(&context, &found(Some((4, 9))))),
            Some(HistoryOf::Lines {
                path: String::from("src/app.rs"),
                lines: LineRange { start: 4, end: 9 },
                commit: commit('a'),
                in_editor: None,
            })
        );
    }

    #[test]
    fn file_history_lines__says_the_lines_in_the_editor_when_they_moved() {
        let context = context(state(), selected(5, 10));
        let Some(HistoryOf::Lines {
            lines: range,
            in_editor,
            ..
        }) = shown(lines(&context, &found(Some((4, 9)))))
        else {
            panic!("shows the history of lines");
        };
        assert_eq!(range, LineRange { start: 4, end: 9 });
        assert_eq!(in_editor, Some(LineRange { start: 5, end: 10 }));
    }

    #[test]
    fn file_history_lines__refuses_lines_typed_since_the_last_commit() {
        let one = context(state(), selected(3, 3));
        let Outcome::Refuse(refusal) = lines(&one, &found(None)) else {
            panic!("refuses a line added since");
        };
        assert_eq!(
            refusal.message.as_str(),
            "This line was added after the last commit, and has no history yet."
        );
        let two = context(state(), selected(3, 4));
        let Outcome::Refuse(refusal) = lines(&two, &found(None)) else {
            panic!("refuses lines added since");
        };
        assert_eq!(
            refusal.message.as_str(),
            "These lines were added after the last commit, and have no history yet."
        );
        let none = Answers::new().with(COMMITTED, Answer::CommittedLines(CommittedLines::InNoCommit));
        let Outcome::Refuse(refusal) = lines(&one, &none) else {
            panic!("refuses a file in no commit");
        };
        assert_eq!(
            refusal.message.as_str(),
            "app.rs is in no commit yet, so its lines have no history."
        );
    }

    #[test]
    fn file_history_lines__looks_for_a_file_renamed_in_the_index_under_its_old_name() {
        let mut state = state();
        state.working_tree.staged.push(renamed("src/app.rs", "src/old.rs"));
        let context = context(state, selected(1, 1));
        let Outcome::Read(_, Query::CommittedLines(query)) = lines(&context, &Answers::new()) else {
            panic!("reads where the lines are");
        };
        assert_eq!(query.renamed_from.as_deref(), Some("src/old.rs"));
    }
}
