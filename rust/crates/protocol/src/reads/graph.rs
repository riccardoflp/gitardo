//! The reads of the graph, from stage 7: a page of the commits the timeline lists (`graph.read-commits`), where a
//! commit is in them (`graph.read-position`) and what a search of the whole history finds (`search.read-matches`),
//! each asked with the timeline the graph shows, as `TimelineRequest` of `engine::timeline` says it.

use std::ffi::OsString;

use engine::timeline::TimelineRequest;
use git::read::file_history::history_arguments;
use git::read::graph::{CommitPage, Parents};
use git::read::search::search_arguments;
use model::file_history::HistoryOf;
use model::filter::{BranchesShown, Kinds};
use model::graph::{PageSize, Row};
use model::search::{Matches, SearchMode};
use serde::{Deserialize, Serialize};

use super::{CommitJson, LineRangeJson, commit_of, id, ids};
use crate::feature::NotACommit;

/// The branches the graph shows (`BranchesShown` of `model`); none in a timeline is every branch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BranchesShownJson {
    /// What is checked out.
    Current,
    /// What is checked out and the upstream of its branch.
    Upstream,
    /// Some kinds of references, and what is checked out.
    Kinds {
        /// The local branches.
        local: bool,
        /// The remote branches.
        remote: bool,
        /// The tags.
        tags: bool,
    },
    /// References ticked one by one, by their full names.
    References {
        /// Their full names, `refs/heads/main`.
        names: Vec<String>,
    },
}

/// Which parents of each commit the graph follows: `"all"` or `"first"` (`gitardo.graph.firstParent`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ParentsJson {
    /// All of them.
    #[default]
    All,
    /// Only the first.
    First,
}

/// How a search of the whole history looks at the commits (`SearchMode` of `model`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SearchModeJson {
    /// The text is in the message, ignoring case.
    Message,
    /// The text is in the name or the address of the author, ignoring case.
    Author,
    /// The commit adds or removes the text.
    Changes,
    /// A line the commit changed matches the text, a regular expression.
    ChangesPattern,
}

/// What narrows the history the graph lists: the history of a file, of some lines or of a folder (`HistoryOf` of
/// `model`), or a search shown as a list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum NarrowingJson {
    /// The commits that changed a file, under every name it had.
    File {
        /// Its name now, from the top of the working tree.
        path: String,
        /// The names it had before, newest first.
        #[serde(default)]
        earlier: Vec<String>,
    },
    /// The commits that changed some lines of a file, followed back from a commit.
    Lines {
        /// The name of the file in that commit.
        path: String,
        /// The lines, as that commit has them.
        lines: LineRangeJson,
        /// The commit the lines are followed back from, by its full id.
        commit: String,
        /// The lines selected in the editor, when they are not the lines of the commit.
        #[serde(default)]
        in_editor: Option<LineRangeJson>,
    },
    /// The commits that changed anything in a folder.
    Folder {
        /// The folder, from the top of the working tree.
        path: String,
    },
    /// The commits a search finds, shown as the list (`search.list-mode`).
    Search {
        /// How the search looks at the commits.
        mode: SearchModeJson,
        /// What it looks for, as typed.
        text: String,
    },
}

/// The timeline a read of the graph is asked about (`TimelineRequest` of `engine::timeline`), and how much of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineJson {
    /// The branches shown; none for every branch.
    #[serde(default)]
    pub shown: Option<BranchesShownJson>,
    /// Which parents are followed.
    #[serde(default)]
    pub parents: ParentsJson,
    /// The commit whose history alone is listed, by its full id, in place of the branches shown.
    #[serde(default)]
    pub alone: Option<String>,
    /// What narrows the history further.
    #[serde(default)]
    pub narrowing: Option<NarrowingJson>,
    /// How many commits a page has, `gitardo.graph.pageSize`: between 100 and 10,000.
    pub page_size: u32,
    /// How many pages are read, from the newest commit; one at least.
    #[serde(default = "one")]
    pub pages: usize,
    /// How many of the newest commits of those pages the client has already: the read starts after them.
    #[serde(default)]
    pub skip: usize,
    /// Whether the stashes are placed among the commits (`gitardo.graph.showStashes`); never while something narrows
    /// the history or one commit's history is listed alone.
    #[serde(default)]
    pub stashes: bool,
}

/// One page at least.
fn one() -> usize {
    1
}

impl From<SearchModeJson> for SearchMode {
    fn from(mode: SearchModeJson) -> SearchMode {
        match mode {
            SearchModeJson::Message => SearchMode::Message,
            SearchModeJson::Author => SearchMode::Author,
            SearchModeJson::Changes => SearchMode::Changes,
            SearchModeJson::ChangesPattern => SearchMode::ChangesPattern,
        }
    }
}

impl From<&BranchesShownJson> for BranchesShown {
    fn from(shown: &BranchesShownJson) -> BranchesShown {
        match shown {
            BranchesShownJson::Current => BranchesShown::Current,
            BranchesShownJson::Upstream => BranchesShown::Upstream,
            BranchesShownJson::Kinds { local, remote, tags } => BranchesShown::Kinds(Kinds {
                local: *local,
                remote: *remote,
                tags: *tags,
            }),
            BranchesShownJson::References { names } => BranchesShown::References(names.clone()),
        }
    }
}

impl NarrowingJson {
    /// The history of a file, of some lines or of a folder this narrowing is; none for a search.
    ///
    /// # Errors
    ///
    /// When the commit of some lines is not a full id.
    pub fn history(&self) -> Result<Option<HistoryOf>, NotACommit> {
        Ok(match self {
            NarrowingJson::File { path, earlier } => Some(HistoryOf::File {
                path: path.clone(),
                earlier: earlier.clone(),
            }),
            NarrowingJson::Lines {
                path,
                lines,
                commit,
                in_editor,
            } => Some(HistoryOf::Lines {
                path: path.clone(),
                lines: (*lines).into(),
                commit: commit_of(commit)?,
                in_editor: in_editor.map(Into::into),
            }),
            NarrowingJson::Folder { path } => Some(HistoryOf::Folder { path: path.clone() }),
            NarrowingJson::Search { .. } => None,
        })
    }

    /// The arguments of `git log` that narrow the history as this does, as the timeline of the desktop gives them.
    ///
    /// # Errors
    ///
    /// When the commit of some lines is not a full id.
    pub fn arguments(&self) -> Result<Vec<OsString>, NotACommit> {
        Ok(match (self, self.history()?) {
            (NarrowingJson::Search { mode, text }, _) => search_arguments((*mode).into(), text),
            (_, Some(history)) => history_arguments(&history),
            (_, None) => Vec::new(),
        })
    }
}

impl TryFrom<&TimelineJson> for TimelineRequest {
    type Error = NotACommit;

    /// The timeline as the engine reads it. The commit of some lines is listed alone, as the desktop lists it, when no
    /// other commit is.
    fn try_from(timeline: &TimelineJson) -> Result<TimelineRequest, NotACommit> {
        let history = timeline
            .narrowing
            .as_ref()
            .map(NarrowingJson::history)
            .transpose()?
            .flatten();
        let alone = match &timeline.alone {
            Some(commit) => Some(commit_of(commit)?),
            None => history.as_ref().and_then(HistoryOf::alone).cloned(),
        };
        let narrowing = timeline
            .narrowing
            .as_ref()
            .map(NarrowingJson::arguments)
            .transpose()?
            .unwrap_or_default();
        Ok(TimelineRequest {
            page_size: PageSize::from_setting(Some(f64::from(timeline.page_size))),
            pages: timeline.pages.max(1),
            parents: match timeline.parents {
                ParentsJson::All => Parents::All,
                ParentsJson::First => Parents::First,
            },
            shows_stashes: timeline.stashes && narrowing.is_empty() && alone.is_none(),
            shown: timeline.shown.as_ref().map(BranchesShown::from),
            narrowing,
            alone,
        })
    }
}

/// The parameters of `read/graph.read-commits`: the timeline, beside `repository`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitsParams {
    /// The repository.
    pub repository: String,
    /// The timeline and how much of it.
    #[serde(flatten)]
    pub timeline: TimelineJson,
}

/// One row of the graph (`Row` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RowJson {
    /// A commit of the history listed.
    Commit {
        /// The commit.
        commit: CommitJson,
    },
    /// A stash, with only its first parent, the commit it was made on.
    Stash {
        /// The commit git keeps the stash as.
        commit: CommitJson,
        /// Its place in the list of stashes: 0 for `stash@{0}`.
        index: usize,
    },
    /// The changes not committed, above the commit checked out; a read of the graph never lists them.
    Uncommitted {
        /// The commit checked out; none before the first commit.
        parent: Option<String>,
        /// How many files have a change of any kind.
        files: usize,
    },
}

impl From<&Row> for RowJson {
    fn from(row: &Row) -> RowJson {
        match row {
            Row::Commit(commit) => RowJson::Commit {
                commit: CommitJson::from(commit),
            },
            Row::Stash { commit, index } => RowJson::Stash {
                commit: CommitJson::from(commit),
                index: *index,
            },
            Row::Uncommitted { parent, files } => RowJson::Uncommitted {
                parent: parent.as_ref().map(id),
                files: *files,
            },
        }
    }
}

/// The answer of `read/graph.read-commits` (`CommitPage` of `git::read::graph`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitPageJson {
    /// The rows, newest first, after those the client has.
    pub rows: Vec<RowJson>,
    /// Whether the history has more commits than the pages asked for.
    pub has_more: bool,
}

impl From<&CommitPage> for CommitPageJson {
    fn from(page: &CommitPage) -> CommitPageJson {
        CommitPageJson {
            rows: page.rows.iter().map(RowJson::from).collect(),
            has_more: page.has_more,
        }
    }
}

/// The parameters of `read/graph.read-position`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionParams {
    /// The repository.
    pub repository: String,
    /// The commit looked for, by its full id.
    pub commit: String,
    /// The timeline it is looked for in; how much of it is read does not matter.
    pub timeline: TimelineJson,
}

/// The answer of `read/graph.read-position`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionResult {
    /// How many commits come before it in the timeline; none when it is not there.
    pub index: Option<usize>,
}

/// The parameters of `read/search.read-matches`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchesParams {
    /// The repository.
    pub repository: String,
    /// What to look for, as typed.
    pub text: String,
    /// How to look at the commits.
    pub mode: SearchModeJson,
    /// The timeline whose history is searched; how much of it is read does not matter.
    pub timeline: TimelineJson,
}

/// The answer of `read/search.read-matches` (`Matches` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchesJson {
    /// The commits found, newest first, at most 5,000.
    pub commits: Vec<String>,
    /// Whether the search found more than those.
    pub has_more: bool,
}

impl From<&Matches> for MatchesJson {
    fn from(matches: &Matches) -> MatchesJson {
        MatchesJson {
            commits: ids(&matches.commits),
            has_more: matches.has_more,
        }
    }
}
