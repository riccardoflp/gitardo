//! The commits of what is checked out, as the Commits pane lists them, and what is pushed of them.

use crate::CommitId;
use crate::commit::Commit;
use crate::contract::Text;

/// The commits of `HEAD`, newest first, and which of them are not pushed yet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BranchLog {
    /// The commits, newest first, as many as were asked for at most.
    pub commits: Vec<Commit>,
    /// Whether `HEAD` has more commits than those listed.
    pub has_more: bool,
    /// The upstream of the branch checked out, in full: `refs/remotes/origin/main`. None without an upstream, when its
    /// branch is gone, or when nothing is checked out.
    pub upstream: Option<String>,
    /// Whether the repository has a remote at all.
    pub has_remote: bool,
    /// The commits listed that are not pushed: those not in the upstream, or, without an upstream, those in no remote
    /// branch. Empty when nothing can be told, as in a repository with no remote.
    pub unpushed: Vec<CommitId>,
}

/// What [`push_state`] says it from: the commits read, the branch checked out, and how far behind its upstream it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BranchLogAndBranch<'a> {
    /// What `commits.read-log` read.
    pub log: &'a BranchLog,
    /// The branch checked out, by its short name; none when `HEAD` is detached.
    pub branch: Option<&'a str>,
    /// How many commits the upstream has that the branch has not.
    pub behind: u32,
}

/// The line above the commits of the Commits pane, and the mark of each commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushState {
    /// The line above the list, or **No commits yet.** in its place.
    pub summary: Text,
    /// One mark per commit listed, in the same order.
    pub marks: Vec<PushMark>,
}

/// The cloud beside a commit, with its title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushMark {
    /// No cloud: there is no remote to push to.
    Unmarked,
    /// A cloud with an arrow, in color: the commit is not pushed.
    Unpushed(Text),
    /// A plain cloud: the commit is pushed.
    Pushed(Text),
}

/// What the line above the commits of the Commits pane says, and the mark of each commit: what is to push, where, and
/// what is to pull.
///
/// Feature: commits.push-state
#[must_use]
pub fn push_state(input: &BranchLogAndBranch<'_>) -> PushState {
    let log = input.log;
    if log.commits.is_empty() {
        return PushState {
            summary: Text::from("No commits yet."),
            marks: Vec::new(),
        };
    }
    if !log.has_remote {
        return PushState {
            summary: Text::from("No remote to push to"),
            marks: vec![PushMark::Unmarked; log.commits.len()],
        };
    }
    let upstream = log
        .upstream
        .as_deref()
        .map(|name| name.strip_prefix("refs/remotes/").unwrap_or(name));
    PushState {
        summary: Text::from(summary(input, upstream)),
        marks: marks(log, upstream),
    }
}

/// The line above the list, when there are commits and a remote.
fn summary(input: &BranchLogAndBranch<'_>, upstream: Option<&str>) -> String {
    let log = input.log;
    let count = log.unpushed.len();
    let is_partial = log.has_more && count == log.commits.len();
    let Some(upstream) = upstream else {
        let reason = match input.branch {
            Some(branch) => format!("{branch} has no upstream"),
            None => String::from("no branch is checked out"),
        };
        return if count == 0 {
            format!("Everything is on a remote branch: {reason}")
        } else {
            format!("{} on no remote branch: {reason}", commits(count, is_partial))
        };
    };
    let pushed = if count == 0 {
        format!("Everything is pushed to {upstream}")
    } else {
        format!("{} to push to {upstream}", commits(count, is_partial))
    };
    if input.behind == 0 {
        pushed
    } else {
        format!("{pushed}, {} to pull", commits(input.behind as usize, false))
    }
}

/// `1 commit`, `3 commits`, or `100+ commits` when there are more than those counted.
fn commits(count: usize, is_partial: bool) -> String {
    let plus = if is_partial { "+" } else { "" };
    let noun = if count == 1 && !is_partial { "commit" } else { "commits" };
    format!("{count}{plus} {noun}")
}

/// The mark of each commit, when there is a remote.
fn marks(log: &BranchLog, upstream: Option<&str>) -> Vec<PushMark> {
    log.commits
        .iter()
        .map(|commit| {
            let is_unpushed = log.unpushed.contains(&commit.id);
            match (is_unpushed, upstream) {
                (true, Some(upstream)) => PushMark::Unpushed(Text::from(format!("Not pushed to {upstream}"))),
                (true, None) => PushMark::Unpushed(Text::from("On no remote branch")),
                (false, Some(upstream)) => PushMark::Pushed(Text::from(format!("Pushed: it is in {upstream}"))),
                (false, None) => PushMark::Pushed(Text::from("Pushed: it is in a remote branch")),
            }
        })
        .collect()
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{BranchLog, BranchLogAndBranch, PushMark, push_state};
    use crate::commit::Commit;
    use crate::{CommitId, Timestamp};

    /// A commit whose hash is a digit repeated.
    fn commit(digit: char) -> Commit {
        Commit {
            id: CommitId::parse(&digit.to_string().repeat(40)).unwrap(),
            parents: Vec::new(),
            author_name: String::from("Ann Lee"),
            author_email: String::from("ann@example.com"),
            author_date: Timestamp(0),
            subject: String::from("Fix the parser"),
        }
    }

    /// A log of the commits of the digits, with the ones of `unpushed` not pushed.
    fn log(digits: &str, unpushed: &str, upstream: Option<&str>) -> BranchLog {
        BranchLog {
            commits: digits.chars().map(commit).collect(),
            has_more: false,
            upstream: upstream.map(str::to_owned),
            has_remote: true,
            unpushed: unpushed.chars().map(|digit| commit(digit).id).collect(),
        }
    }

    fn summary(log: &BranchLog, branch: Option<&str>, behind: u32) -> String {
        push_state(&BranchLogAndBranch { log, branch, behind })
            .summary
            .to_string()
    }

    #[test]
    fn commits_push_state__counts_what_is_to_push_and_to_pull() {
        let log = log("12345", "12", Some("refs/remotes/origin/main"));
        assert_eq!(
            summary(&log, Some("main"), 3),
            "2 commits to push to origin/main, 3 commits to pull"
        );
    }

    #[test]
    fn commits_push_state__says_everything_is_pushed_when_level() {
        let log = log("123", "", Some("refs/remotes/origin/main"));
        assert_eq!(summary(&log, Some("main"), 0), "Everything is pushed to origin/main");
        assert_eq!(
            summary(&log, Some("main"), 1),
            "Everything is pushed to origin/main, 1 commit to pull"
        );
    }

    #[test]
    fn commits_push_state__marks_the_commits_on_no_remote_branch_without_an_upstream() {
        let log = log("123", "1", None);
        let state = push_state(&BranchLogAndBranch {
            log: &log,
            branch: Some("feature"),
            behind: 0,
        });
        assert_eq!(
            state.summary.as_str(),
            "1 commit on no remote branch: feature has no upstream"
        );
        assert_eq!(state.marks[0], PushMark::Unpushed("On no remote branch".into()));
        assert_eq!(
            state.marks[1],
            PushMark::Pushed("Pushed: it is in a remote branch".into())
        );
        assert_eq!(
            summary(&self::log("12", "", None), None, 0),
            "Everything is on a remote branch: no branch is checked out"
        );
    }

    #[test]
    fn commits_push_state__marks_nothing_without_a_remote() {
        let mut log = log("12", "12", None);
        log.has_remote = false;
        let state = push_state(&BranchLogAndBranch {
            log: &log,
            branch: Some("main"),
            behind: 0,
        });
        assert_eq!(state.summary.as_str(), "No remote to push to");
        assert_eq!(state.marks, vec![PushMark::Unmarked, PushMark::Unmarked]);
    }

    #[test]
    fn commits_push_state__titles_the_marks_with_the_upstream() {
        let log = log("12", "1", Some("refs/remotes/origin/main"));
        let state = push_state(&BranchLogAndBranch {
            log: &log,
            branch: Some("main"),
            behind: 0,
        });
        assert_eq!(
            state.marks,
            vec![
                PushMark::Unpushed("Not pushed to origin/main".into()),
                PushMark::Pushed("Pushed: it is in origin/main".into()),
            ]
        );
    }

    #[test]
    fn commits_push_state__adds_a_plus_when_every_commit_listed_is_unpushed_and_there_are_more() {
        let mut log = log("12", "12", Some("refs/remotes/origin/main"));
        log.has_more = true;
        assert_eq!(summary(&log, Some("main"), 0), "2+ commits to push to origin/main");
    }

    #[test]
    fn commits_push_state__says_no_commits_yet_with_no_commit() {
        let log = BranchLog::default();
        let state = push_state(&BranchLogAndBranch {
            log: &log,
            branch: Some("main"),
            behind: 0,
        });
        assert_eq!(state.summary.as_str(), "No commits yet.");
        assert_eq!(state.marks, Vec::new());
    }
}
