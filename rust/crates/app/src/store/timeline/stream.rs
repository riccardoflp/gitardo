//! The pages of a read of the timeline as they come (`graph.load-more`, D54): each one is put under the pages before
//! it, and while they are the rows listed already, the rows are left as they are, so that a read that finds the same
//! commits changes none of them.

use model::graph::Row;

use super::super::message::JobId;

/// A read of the timeline whose first page came and whose next pages are coming.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stream {
    /// The job that reads them.
    pub job: JobId,
    /// Where the next page goes among the rows read from git: after the rows of the pages of the read so far.
    pub at: usize,
    /// Whether the pages so far are the rows listed already.
    pub is_same: bool,
}

impl Stream {
    /// A read that starts at the top of the history, whose pages are compared with the rows listed.
    #[must_use]
    pub fn from_top(job: JobId) -> Stream {
        Stream {
            job,
            at: 0,
            is_same: true,
        }
    }

    /// A read that goes on under the rows listed, which are kept.
    #[must_use]
    pub fn under(job: JobId, rows: usize) -> Stream {
        Stream {
            job,
            at: rows,
            is_same: false,
        }
    }

    /// Puts a page where the read stands among the rows read from git: rows listed already, when they are the same
    /// rows, stay; otherwise the rows from there on are those of the page. Says whether the rows changed.
    pub fn take(&mut self, rows: &mut Vec<Row>, page: Vec<Row>) -> bool {
        let end = self.at + page.len();
        let is_listed = self.is_same && rows.get(self.at..end).is_some_and(|listed| listed == page.as_slice());
        let start = self.at;
        self.at = end;
        if is_listed {
            return false;
        }
        self.is_same = false;
        rows.truncate(start);
        rows.extend(page);
        true
    }

    /// The end of the read: the rows listed under those of its pages are dropped when the read went to the end of the
    /// history, and kept when it stopped at its last commit with more after it. Says whether the rows changed.
    #[must_use]
    pub fn finish(&self, rows: &mut Vec<Row>, has_more: bool) -> bool {
        if has_more || rows.len() <= self.at {
            return false;
        }
        rows.truncate(self.at);
        true
    }
}

#[cfg(test)]
mod tests {
    use model::commit::Commit;
    use model::graph::Row;
    use model::{CommitId, Timestamp};

    use super::Stream;
    use crate::store::JobId;

    /// A line of commits, each the parent of the one before, with a word in their subjects.
    fn line(count: usize, word: &str) -> Vec<Row> {
        let id = |number: usize| CommitId::parse(&format!("{number:040x}")).unwrap();
        (0..count)
            .map(|number| {
                Row::Commit(Commit {
                    id: id(number + 1),
                    parents: if number + 1 < count {
                        vec![id(number + 2)]
                    } else {
                        Vec::new()
                    },
                    author_name: String::from("Test Author"),
                    author_email: String::from("author@example.com"),
                    author_date: Timestamp(i64::try_from(count - number).unwrap()),
                    subject: format!("{word} {number}"),
                })
            })
            .collect()
    }

    #[test]
    fn the_same_rows_in_pages_change_nothing_and_a_shorter_history_drops_the_rest() {
        let listed = line(10, "Commit");
        let mut rows = listed.clone();
        let mut stream = Stream::from_top(JobId(1));
        assert!(!stream.take(&mut rows, listed.get(..4).unwrap().to_vec()));
        assert!(!stream.take(&mut rows, listed.get(4..8).unwrap().to_vec()));
        assert_eq!(rows, listed);
        assert!(
            !stream.finish(&mut rows, true),
            "a read stopped with more keeps the rest"
        );
        assert!(stream.finish(&mut rows, false));
        assert_eq!(rows, listed.get(..8).unwrap());
    }

    #[test]
    fn a_page_that_differs_replaces_the_rows_from_there_on() {
        let listed = line(10, "Commit");
        let other = line(12, "Other");
        let mut rows = listed.clone();
        let mut stream = Stream::from_top(JobId(1));
        assert!(stream.take(&mut rows, other.get(..5).unwrap().to_vec()));
        assert!(stream.take(&mut rows, other.get(5..).unwrap().to_vec()));
        assert_eq!(rows, other);
    }
}
