//! The badge of a pull request at the end of the label of its branch (`graph.pull-request-badge`): its number, with the
//! icon of a pull request or of a draft, in the color of its checks; a click opens it on GitHub.

use model::github::{Checks, PullRequest};
use model::graph::{Label, LabelKind};
use ui::{AccessRole, Badge, BadgeFill, Description, Icon, Rect, Sense, Tone, Ui};

use super::state::HistoryState;
use crate::icons;
use crate::intent::Intent;
use crate::text::github as words;

/// A badge to draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PullRequestBadge<'a> {
    /// The pull request.
    pub pull_request: &'a PullRequest,
    /// The lane of the label, whose color the badge has when the pull request has no checks.
    pub lane: usize,
}

/// The icon of a pull request, and of a draft.
const PULL_REQUEST: [Icon; 1] = [icons::PULL_REQUEST];
/// The icon of a draft pull request.
const DRAFT: [Icon; 1] = [icons::DRAFT_PULL_REQUEST];

impl PullRequestBadge<'_> {
    /// The tooltip: `Pull request #12: Add the parser · checks passed`, then **Click to open it on GitHub**.
    #[must_use]
    pub fn tooltip(&self) -> String {
        let pull_request = self.pull_request;
        let first = words::badge_tooltip(
            pull_request.number,
            &pull_request.title,
            pull_request.is_draft,
            pull_request.checks,
        );
        format!("{first}\n{}", words::CLICK_TO_OPEN)
    }

    /// The pill, with its words.
    fn badge<'text>(&self, text: &'text str) -> Badge<'text> {
        Badge {
            text,
            icons: if self.pull_request.is_draft {
                &DRAFT
            } else {
                &PULL_REQUEST
            },
            tone: match self.pull_request.checks {
                Some(Checks::Passed) => Tone::Success,
                Some(Checks::Failed) => Tone::Error,
                Some(Checks::Running) => Tone::Warning,
                None => Tone::Lane(self.lane),
            },
            fill: BadgeFill::Tint,
        }
    }
}

/// How wide the badge is.
#[must_use]
pub fn pull_request_badge_width(ui: &Ui<'_>, state: &PullRequestBadge<'_>) -> f32 {
    let text = words::badge_text(state.pull_request.number);
    ui.badge_width(&state.badge(&text))
}

/// The tone of the badge: the color of its checks, or of its lane without checks.
#[must_use]
pub fn pull_request_badge_tone(state: &PullRequestBadge<'_>) -> Tone {
    state.badge("").tone
}

/// Draws the badge at the start of the rectangle its caller clipped it to, with its tooltip; a click opens the pull
/// request in the browser, and is not a click on the row under it.
///
/// Feature: graph.pull-request-badge
pub fn pull_request_badge_view(state: &PullRequestBadge<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let drawn = draw_pull_request_badge(ui, rect, state);
    let id = ui.id(("pull request badge", state.pull_request.number));
    let response = ui.interact(id, drawn, Sense::CLICK | Sense::HOVER);
    let tooltip = state.tooltip();
    ui.describe(response.id, drawn, &Description::new(AccessRole::Button, &tooltip));
    ui.tooltip(&response, &tooltip);
    if response.clicked {
        return vec![Intent::OpenUrl(state.pull_request.url.clone())];
    }
    Vec::new()
}

/// Draws the badge alone, with nothing to click: in a row of the branch rail, whose tooltip says the pull request.
pub fn draw_pull_request_badge(ui: &mut Ui<'_>, rect: Rect, state: &PullRequestBadge<'_>) -> Rect {
    let text = words::badge_text(state.pull_request.number);
    ui.badge(rect, &state.badge(&text))
}

/// The room the badge of the pull request of a label takes after it, with the gap before it: none when its branch has
/// no pull request.
pub(super) fn badge_room(ui: &Ui<'_>, state: &HistoryState<'_>, label: &Label, lane: usize) -> f32 {
    of_label(state, label).map_or(0.0, |pull_request| {
        let badge = PullRequestBadge { pull_request, lane };
        ui.theme().sizes.spacing.medium + pull_request_badge_width(ui, &badge)
    })
}

/// The badge after a label of the timeline, when its branch has a pull request and there is room for it before `end`:
/// where what follows starts.
pub(super) fn after_label(
    ui: &mut Ui<'_>,
    state: &HistoryState<'_>,
    (labels, index): (&[Label], usize),
    (drawn, lane, end): (Rect, usize, f32),
    intents: &mut Vec<Intent>,
) -> f32 {
    let gap = ui.theme().sizes.spacing.medium;
    let next = drawn.right() + gap;
    let Some(pull_request) = labels.get(index).and_then(|label| of_label(state, label)) else {
        return next;
    };
    let state = PullRequestBadge { pull_request, lane };
    let width = pull_request_badge_width(ui, &state);
    if next + width > end.min(ui.current_clip().right()) {
        return next;
    }
    let cell = Rect::new(next, drawn.y, width, drawn.height);
    intents.extend(ui.clip(cell, |ui| pull_request_badge_view(&state, ui)));
    next + width + gap
}

/// The pull request of the branch of a label: a local branch has the one of its upstream; a remote branch has one on
/// its own label only when no local branch tracks it.
fn of_label<'a>(state: &HistoryState<'a>, label: &Label) -> Option<&'a PullRequest> {
    let menus = state.references?;
    let repository = menus.repository;
    match &label.kind {
        LabelKind::Branch { .. } => {
            let branch = repository.branches.iter().find(|branch| branch.name == label.name)?;
            menus.github.of_branch(branch)
        }
        LabelKind::RemoteBranch => {
            let remote = repository
                .remote_branches
                .iter()
                .find(|remote| remote.name == label.name)?;
            menus.github.of_remote_branch(repository, remote)
        }
        _ => None,
    }
}
