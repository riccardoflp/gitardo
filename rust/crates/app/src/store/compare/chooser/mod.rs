//! The comparison chooser as the store keeps it (`compare.chooser`): the two sides held, the box being chosen, what is
//! typed and what git says of it, the sides offered, and what `compare.summary` says once both are held; **Compare**
//! runs `compare.choose`, or `compare.change-side` when it was opened from a side of a comparison page, with the sides
//! as answers, so that the operation asks nothing.

use std::path::PathBuf;
use std::sync::Arc;

use model::CommitId;
use model::compare::{
    CompareSide, ComparisonPart, ComparisonTarget, RecentComparison, Side, SummaryInput, initial_mode, summary,
};
use model::contract::{Answer, AnswerKey, Answers, FeatureId, Target};
use model::state::RepositoryState;
use views::compare::{ChooserBox, ChooserGroup, ChooserIntent, ChooserSide, ChooserState, ComparePageId, TypedCommit};
use views::feature::FeatureIntent;
use views::text::compare as words;

use crate::store::message::{Effect, JobId, JobKind};
use crate::store::search::SearchJob;
use crate::store::state::State;
use crate::work::compare::ComparisonRead;
use rows::rows_of;

mod rows;

/// The comparison chooser, while it is open.
#[derive(Clone, Debug, PartialEq)]
pub struct Chooser {
    /// The folder of the repository it chooses in.
    pub folder: PathBuf,
    /// That repository, as last read.
    pub repository: Arc<RepositoryState>,
    /// Its title: what it was opened for.
    pub title: &'static str,
    /// What **From** holds.
    pub from: Option<ChooserSide>,
    /// What **To** holds.
    pub to: Option<ChooserSide>,
    /// The box being chosen.
    pub choosing: ChooserBox,
    /// What is typed.
    pub typed: String,
    /// The commit git says it stands for, once it said: none inside when git knows none.
    pub typed_commit: Option<Option<CommitId>>,
    /// The read of what is typed, while it runs.
    pub typed_job: Option<JobId>,
    /// Every side offered, in the order of their headings.
    pub rows: Vec<ChooserSide>,
    /// The row lit, among those offered now.
    pub lit: usize,
    /// The number of the last change of the row lit or of the box chosen.
    pub serial: u64,
    /// The comparisons opened before in its repository.
    pub recent: Vec<RecentComparison>,
    /// What `compare.summary` says of the two sides held.
    pub summary: Option<String>,
    /// The read of the two sides held, while it runs.
    pub summary_job: Option<JobId>,
    /// The comparison page whose side it chooses (`compare.change-side`).
    pub changing: Option<ComparePageId>,
}

/// **Compare branches, tags or commits…**: the chooser over the repository shown, **From** holding what is checked out
/// and **To** being chosen.
pub fn open(state: &mut State) -> Vec<Effect> {
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let (folder, repository) = (read.folder.clone(), Arc::clone(&read.state));
    let rows = rows_of(state, (&folder, &repository));
    let from = rows
        .iter()
        .find(|row| row.group == ChooserGroup::CheckedOut && !row.side.is_working_tree())
        .cloned();
    let choosing = if from.is_some() {
        ChooserBox::To
    } else {
        ChooserBox::From
    };
    let recent = super::recent_of(state, &folder);
    state.compares.chooser = Some(Chooser {
        folder,
        repository,
        title: words::CHOOSER_TITLE,
        from,
        to: None,
        choosing,
        typed: String::new(),
        typed_commit: None,
        typed_job: None,
        rows,
        lit: 0,
        serial: 0,
        recent,
        summary: None,
        summary_job: None,
        changing: None,
    });
    Vec::new()
}

/// A click on a side of a comparison page: the chooser holding both sides of the page, the side clicked being chosen,
/// among the choices of the page's repository.
pub fn open_for_page(state: &mut State, id: ComparePageId, is_first: bool) -> Vec<Effect> {
    let Some(page) = state.compares.pages.iter().find(|page| page.id == id) else {
        return Vec::new();
    };
    let (folder, repository) = (page.folder.clone(), Arc::clone(&page.repository));
    let held = |side: &CompareSide| ChooserSide {
        group: if side.is_working_tree() {
            ChooserGroup::CheckedOut
        } else if side.is_followed {
            ChooserGroup::Branches
        } else {
            ChooserGroup::Typed
        },
        side: side.clone(),
        subject: String::new(),
        date: None,
    };
    let (from, to) = (held(&page.target.first), held(&page.target.second));
    let rows = rows_of(state, (&folder, &repository));
    let recent = super::recent_of(state, &folder);
    state.compares.chooser = Some(Chooser {
        folder,
        repository,
        title: words::CHANGE_SIDE_TITLE,
        from: Some(from),
        to: Some(to),
        choosing: if is_first { ChooserBox::From } else { ChooserBox::To },
        typed: String::new(),
        typed_commit: None,
        typed_job: None,
        rows,
        lit: 0,
        serial: 0,
        recent,
        summary: None,
        summary_job: None,
        changing: Some(id),
    });
    read_summary(state)
}

/// Something asked of the chooser.
pub fn reduce(state: &mut State, intent: ChooserIntent) -> Vec<Effect> {
    let Some(chooser) = state.compares.chooser.as_mut() else {
        return Vec::new();
    };
    chooser.serial += 1;
    match intent {
        ChooserIntent::Focus(which) => {
            chooser.choosing = which;
            chooser.lit = 0;
            Vec::new()
        }
        ChooserIntent::Swap => {
            std::mem::swap(&mut chooser.from, &mut chooser.to);
            read_summary(state)
        }
        ChooserIntent::Type(text) => typed(state, text),
        ChooserIntent::Light(row) => {
            chooser.lit = row;
            Vec::new()
        }
        ChooserIntent::Choose(side) => {
            let which = chooser.choosing;
            let other = match which {
                ChooserBox::From => &chooser.to,
                ChooserBox::To => &chooser.from,
            };
            if other.is_none() {
                chooser.choosing = which.other();
            }
            match which {
                ChooserBox::From => chooser.from = Some(*side),
                ChooserBox::To => chooser.to = Some(*side),
            }
            chooser.lit = 0;
            let mut effects = typed(state, String::new());
            effects.extend(read_summary(state));
            effects
        }
        ChooserIntent::Compare => compare(state),
        ChooserIntent::OpenRecent(place) => {
            let mut effects = close(state);
            let answers = Answers::new().with(features::compare::FIRST, Answer::Choice(format!("recent:{place}")));
            let run = FeatureIntent::RunWith(FeatureId("compare.choose"), Target::Head, answers);
            effects.extend(crate::store::feature::reduce(state, run));
            effects
        }
        ChooserIntent::Close => close(state),
    }
}

/// Text typed: the row lit goes back to the first, and git is asked which commit it stands for, unless it is empty or
/// starts with `-`, which git would read as an option.
fn typed(state: &mut State, text: String) -> Vec<Effect> {
    let Some(chooser) = state.compares.chooser.as_mut() else {
        return Vec::new();
    };
    let running = chooser.typed_job.take();
    chooser.typed_commit = None;
    chooser.lit = 0;
    let revision = text.trim().to_owned();
    chooser.typed = text;
    let folder = chooser.repository.folder.clone();
    let mut effects = cancel(state, running);
    if revision.is_empty() || revision.starts_with('-') {
        return effects;
    }
    let job = crate::store::search::jobs::start(
        state,
        SearchJob::Revision {
            folder,
            request: git::read::search::RevisionRequest { revision },
        },
    );
    if let Some(chooser) = state.compares.chooser.as_mut() {
        chooser.typed_job = Some(job.id);
    }
    effects.push(Effect::Start(job));
    effects
}

/// Whether a read of the search is the one of what is typed in the chooser.
#[must_use]
pub fn is_typed_job(state: &State, job: JobId) -> bool {
    state
        .compares
        .chooser
        .as_ref()
        .is_some_and(|chooser| chooser.typed_job == Some(job))
}

/// What git says of the text typed: the commit it stands for, or none.
pub fn revision_read(state: &mut State, result: Result<Option<CommitId>, String>) -> Vec<Effect> {
    if let Some(chooser) = state.compares.chooser.as_mut() {
        chooser.typed_job = None;
        match result {
            Ok(commit) => chooser.typed_commit = Some(commit),
            Err(error) => state.last_error = Some(error),
        }
    }
    Vec::new()
}

/// Reads the two sides held, for what `compare.summary` says of them; nothing while a box is empty.
fn read_summary(state: &mut State) -> Vec<Effect> {
    let Some(chooser) = state.compares.chooser.as_mut() else {
        return Vec::new();
    };
    chooser.summary = None;
    let running = chooser.summary_job.take();
    let sides = chooser.from.as_ref().zip(chooser.to.as_ref());
    let target = sides.map(|(from, to)| ComparisonTarget {
        first: from.side.clone(),
        second: to.side.clone(),
        mode: initial_mode(&(from.side.clone(), to.side.clone())),
        path: None,
        merge_base: None,
        part: ComparisonPart::Whole,
    });
    let folder = chooser.folder.clone();
    let mut effects = cancel(state, running);
    let Some(target) = target.filter(|target| !(target.first.is_working_tree() && target.second.is_working_tree()))
    else {
        return effects;
    };
    let kind = JobKind::ReadComparison {
        folder,
        target: Box::new(target),
    };
    let job = super::super::workspace::start_job(state, kind);
    if let Some(chooser) = state.compares.chooser.as_mut() {
        chooser.summary_job = Some(job.id);
    }
    effects.push(Effect::Start(job));
    effects
}

/// The read of the two sides held: what `compare.summary` says of how far apart they are.
pub fn summary_read(state: &mut State, result: Result<Box<ComparisonRead>, String>) {
    let Some(chooser) = state.compares.chooser.as_mut() else {
        return;
    };
    chooser.summary_job = None;
    match result {
        Ok(read) => {
            let said = summary(&SummaryInput {
                first: &read.first,
                second: &read.second,
                view: &read.view,
            });
            chooser.summary = Some(said.distance);
        }
        Err(error) => state.last_error = Some(error),
    }
}

/// Cancels a read of the chooser.
fn cancel(state: &mut State, job: Option<JobId>) -> Vec<Effect> {
    job.into_iter()
        .map(|job| {
            let _cancelled = state.jobs.remove(&job);
            Effect::Cancel(job)
        })
        .collect()
}

/// Closes the chooser, cancelling its reads.
fn close(state: &mut State) -> Vec<Effect> {
    let Some(chooser) = state.compares.chooser.take() else {
        return Vec::new();
    };
    let mut effects = cancel(state, chooser.typed_job);
    effects.extend(cancel(state, chooser.summary_job));
    effects
}

/// **Compare**: `compare.choose` with both sides answered; from a comparison page, `compare.change-side` with the side
/// changed when only one is, and the new pair put in the same page in any case.
fn compare(state: &mut State) -> Vec<Effect> {
    let Some(chooser) = state.compares.chooser.as_ref() else {
        return Vec::new();
    };
    let (Some(from), Some(to)) = (chooser.from.clone(), chooser.to.clone()) else {
        return Vec::new();
    };
    let changing = chooser.changing;
    let repository = (chooser.folder.clone(), Arc::clone(&chooser.repository));
    let mut effects = close(state);
    let both = side_answers(
        side_answers(Answers::new(), (&from.side, SIDE_KEYS[0])),
        (&to.side, SIDE_KEYS[1]),
    );
    let Some(id) = changing else {
        let run = (FeatureId("compare.choose"), Target::Head);
        effects.extend(crate::store::feature::run_in(state, repository, run, both));
        return effects;
    };
    let Some(page) = state.compares.pages.iter().find(|page| page.id == id) else {
        return effects;
    };
    let (is_first_kept, is_second_kept) = (from.side == page.target.first, to.side == page.target.second);
    let (feature, target, answers) = match (is_first_kept, is_second_kept) {
        (true, true) => return effects,
        (true, false) => (
            "compare.change-side",
            page.target_of(ComparisonPart::SecondSide),
            side_answers(Answers::new(), (&to.side, SIDE_KEYS[1])),
        ),
        (false, true) => (
            "compare.change-side",
            page.target_of(ComparisonPart::FirstSide),
            side_answers(Answers::new(), (&from.side, SIDE_KEYS[0])),
        ),
        (false, false) => ("compare.choose", page.target_of(ComparisonPart::Whole), both),
    };
    let run = (FeatureId(feature), Target::Comparison(Box::new(target)));
    let ran = crate::store::feature::run_in(state, repository, run, answers);
    if !ran.is_empty() {
        state.compares.changing = Some(id);
    }
    effects.extend(ran);
    effects
}

/// The keys of the answers of each side: what is picked, the revision typed and its commit.
const SIDE_KEYS: [(AnswerKey, AnswerKey, AnswerKey); 2] = [
    (
        features::compare::FIRST,
        features::compare::FIRST_REVISION,
        features::compare::FIRST_COMMIT,
    ),
    (
        features::compare::SECOND,
        features::compare::SECOND_REVISION,
        features::compare::SECOND_COMMIT,
    ),
];

/// The answers that give a side to `compare.choose` or `compare.change-side`: the working tree, or its name as a
/// revision typed with the commit it stands for, so that the operation names and follows it as the chooser showed it.
fn side_answers(
    answers: Answers,
    (side, (picked, revision, commit)): (&CompareSide, (AnswerKey, AnswerKey, AnswerKey)),
) -> Answers {
    match &side.side {
        Side::WorkingTree => answers.with(picked, Answer::Choice(String::from("working-tree"))),
        Side::Commit(at) => answers
            .with(picked, Answer::Choice(String::from("revision")))
            .with(revision, Answer::Text(side.name.clone()))
            .with(commit, Answer::Commit(Some(at.clone()))),
    }
}

/// What the chooser shows, while it is open.
#[must_use]
pub fn chooser_state(state: &State) -> Option<ChooserState<'_>> {
    let chooser = state.compares.chooser.as_ref()?;
    Some(ChooserState {
        title: chooser.title,
        from: chooser.from.as_ref(),
        to: chooser.to.as_ref(),
        choosing: chooser.choosing,
        typed: &chooser.typed,
        typed_commit: match &chooser.typed_commit {
            None => TypedCommit::Unknown,
            Some(Some(commit)) => TypedCommit::Commit(commit),
            Some(None) => TypedCommit::NoCommit,
        },
        rows: &chooser.rows,
        lit: chooser.lit,
        serial: chooser.serial,
        recent: &chooser.recent,
        summary: chooser.summary.as_deref(),
        now: state.timeline.read_at,
    })
}
