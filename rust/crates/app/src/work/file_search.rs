//! The reads of the Search page on a worker (`file-search.page`): the files of the worktree listed, and a search of
//! them, which ranks the names at once and then hands on the lines of text as git finds them.

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use git::read::file_search::{NamesRequest, TextRequest, read_names, read_text};
use git::{CancelToken, Git, GitError, Taken};
use model::file_search::{
    Found, MOST_FILES_IN_ALL, MOST_FOLDERS_IN_ALL, MOST_LINES, MOST_NAMES, Mode, RankRequest, Search, TextMatch,
    folders_of, names_found, rank_names,
};

use crate::store::{JobId, Message};

/// The files of a worktree, as the search lists them.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Names {
    /// What they were listed for.
    pub request: NamesRequest,
    /// Their paths from the top of the worktree, in order.
    pub files: Vec<String>,
    /// The folders that hold them, in order.
    pub folders: Vec<String>,
}

/// A read of the Search page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchJob {
    /// Lists the files of the worktree (`file-search.read-names`).
    Names(NamesRequest),
    /// Ranks the names and searches the text (`file-search.rank-names`, `file-search.read-text`).
    Find(Box<FindJob>),
}

/// A search among the files listed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindJob {
    /// What is looked for, and where.
    pub search: Search,
    /// The files listed.
    pub names: Arc<Names>,
    /// The files that differ from the last commit, when they alone are searched.
    pub changed: Option<Vec<String>>,
}

/// What a read of the Search page says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchAnswer {
    /// The files listed, or what git said when it could not list them.
    Names(Result<Arc<Names>, String>),
    /// The names that match, the first thing a search says.
    Ranked(Box<Found>),
    /// More lines of text found.
    Text(Vec<TextMatch>),
    /// The search ended: whether there are more lines than those handed on, and what git said when it could not
    /// search, as of a regular expression it cannot read.
    Done {
        /// Whether there are more lines.
        has_more: bool,
        /// What git said when it could not search.
        refused: Option<String>,
    },
}

impl SearchAnswer {
    /// Whether the job that says it has nothing more to say.
    #[must_use]
    pub fn is_last(&self) -> bool {
        matches!(self, SearchAnswer::Names(_) | SearchAnswer::Done { .. })
    }
}

/// Runs a read of the Search page in the top folder of a worktree, and gives `send` each thing it says.
pub fn run(job: JobId, (folder, search): (&Path, &SearchJob), cancel: &CancelToken, send: &mut dyn FnMut(Message)) {
    let git = Git::new(folder).with_cancel(cancel.clone());
    let mut say = |answer: SearchAnswer| send(Message::FileSearch { job, answer });
    match search {
        SearchJob::Names(request) => {
            let names = read_names(&git, request).map(|files| {
                Arc::new(Names {
                    request: request.clone(),
                    folders: folders_of(&files),
                    files,
                })
            });
            say(SearchAnswer::Names(names.map_err(|error| error.to_string())));
        }
        SearchJob::Find(find) => {
            let files = searched_files(find);
            say(SearchAnswer::Ranked(Box::new(ranked(find, &files))));
            let ended = text(&git, find, &files, &mut |lines| say(SearchAnswer::Text(lines)));
            let (has_more, refused) = match ended {
                Ok(has_more) => (has_more, None),
                Err(error) => (false, Some(said(&error))),
            };
            say(SearchAnswer::Done { has_more, refused });
        }
    }
}

/// The files a search looks in: those listed, or those of them that differ from the last commit.
fn searched_files(find: &FindJob) -> Cow<'_, [String]> {
    match &find.changed {
        Some(changed) => {
            let changed: BTreeSet<&str> = changed.iter().map(String::as_str).collect();
            let files = find.names.files.iter().filter(|file| changed.contains(file.as_str()));
            Cow::Owned(files.cloned().collect())
        }
        None => Cow::Borrowed(&find.names.files),
    }
}

/// The names that match what is typed, for the mode: the files, the folders, or fewer of both in **All**, where a
/// name has to match more closely.
fn ranked(find: &FindJob, files: &[String]) -> Found {
    let search = &find.search;
    let folders = match &find.changed {
        Some(_) => Cow::Owned(folders_of(files)),
        None => Cow::Borrowed(&find.names.folders),
    };
    let (most_files, most_folders) = match search.mode {
        Mode::Text => (0, 0),
        Mode::Files => (MOST_NAMES, 0),
        Mode::Folders => (0, MOST_NAMES),
        Mode::All => (MOST_FILES_IN_ALL, MOST_FOLDERS_IN_ALL),
    };
    let rank = |names: &[String], most: usize| {
        if most == 0 {
            return (Vec::new(), 0);
        }
        let request = RankRequest {
            query: &search.query,
            strict: search.mode == Mode::All,
            most,
        };
        let ranked = rank_names(names, &request);
        let total = ranked.total;
        (names_found(names, ranked), total)
    };
    let (files, files_total) = rank(files, most_files);
    let (folders, folders_total) = rank(&folders, most_folders);
    Found {
        folders,
        folders_total,
        files,
        files_total,
        text: Vec::new(),
        more_text: false,
    }
}

/// Searches the text of the files, when the mode lists lines and something is typed, and says whether there are more
/// lines than those handed on.
fn text(
    git: &Git,
    find: &FindJob,
    files: &[String],
    hand_on: &mut dyn FnMut(Vec<TextMatch>),
) -> Result<bool, GitError> {
    let search = &find.search;
    if !search.mode.searches_text() || search.query.trim().is_empty() {
        return Ok(false);
    }
    let names = &find.names.request;
    let request = TextRequest {
        text: search.query.clone(),
        match_case: search.match_case,
        whole_word: search.whole_word,
        regex: search.regex,
        ignored: names.ignored,
        folder: names.folder.clone(),
        pathspecs: names.pathspecs.clone(),
        files: find.changed.as_ref().map(|_| files.to_vec()),
        most: MOST_LINES,
    };
    let ended = read_text(git, &request, &mut |lines| {
        hand_on(lines);
        Taken::More
    })?;
    Ok(ended.more)
}

/// What git said when it could not search, without the word it starts its errors with.
fn said(error: &GitError) -> String {
    let said = error.stderr().map_or_else(|| error.to_string(), str::to_owned);
    let said = said.trim();
    said.strip_prefix("fatal: ").unwrap_or(said).to_owned()
}
