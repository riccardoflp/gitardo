//! A bisect in the timeline: the banner above the commits that says what it needs next and takes the answer for the
//! commit checked out (`bisect.banner`), and the marks of the commits marked so far (`bisect.marks`).

#[cfg(test)]
mod tests;

use model::CommitId;
use model::bisect::Bisect;
use model::contract::{Answer, AnswerKey, Answers, FeatureId, Target};
use model::graph::BisectMark;
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Axis, Button, ButtonKind, Icon, Label, Rect, Tone, Ui};

use crate::feature::FeatureIntent;
use crate::graph::GraphIntent;
use crate::intent::Intent;
use crate::text::bisect as words;

/// The key under which `bisect.answer` takes the button pressed: `good`, `bad` or `skip`.
const ANSWER: AnswerKey = AnswerKey("answer");

/// What the banner of a bisect shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BisectBanner<'a> {
    /// The bisect under way.
    pub bisect: &'a Bisect,
    /// The commit checked out, which git asks to test.
    pub head: Option<&'a CommitId>,
    /// Whether an operation runs: the answers wait for it.
    pub is_running: bool,
}

/// What the banner says the bisect needs next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BannerSays {
    /// No bad commit is marked yet.
    MarkBad,
    /// A bad commit is marked, and no good one.
    MarkGood,
    /// Commits are left to test: the one checked out, and how many can still be the first bad one.
    Test(String),
    /// The first bad commit is found.
    Found(CommitId),
}

impl BisectBanner<'_> {
    /// What the banner says.
    #[must_use]
    pub fn says(&self) -> BannerSays {
        let bisect = self.bisect;
        match (&bisect.bad, bisect.good.is_empty(), bisect.remaining) {
            (None, _, _) => BannerSays::MarkBad,
            (Some(_), true, _) => BannerSays::MarkGood,
            (Some(bad), false, Some(1)) => BannerSays::Found(bad.clone()),
            (Some(_), false, remaining) => BannerSays::Test(words::test(
                self.head.map_or("HEAD", CommitId::short),
                remaining.unwrap_or_default(),
            )),
        }
    }
}

/// The banner of a bisect, over the rectangle its caller clipped it to, above the commits: after a search icon, what
/// the bisect needs next; **Good**, **Bad** and **Skip** while commits are left to test; **Show** once the first bad
/// commit is found; and **End**, always.
///
/// Feature: bisect.banner
pub fn banner_view(state: &BisectBanner<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    ui.fill_rect(rect, theme.colors.card_background);
    let line = Rect::new(
        rect.x,
        rect.bottom() - theme.sizes.border,
        rect.width,
        theme.sizes.border,
    );
    ui.separator(line, Axis::Horizontal);
    let says = state.says();
    let mut intents = Vec::new();
    ui.region("bisect banner", (rect, AccessRole::Group, words::BANNER), |ui| {
        let mut parts = Area::with_gap(
            inset(rect, Insets::symmetric(spacing.large + spacing.small, 0.0)),
            spacing.medium,
        );
        ui.icon_in(parts.left(theme.sizes.icon), Icon::Search, Tone::Accent);
        let buttons = buttons(&says, state.is_running);
        for (label, tooltip, intent) in buttons {
            let button = Button::secondary(label).kind(ButtonKind::Outline);
            let (width, height) = ui.button_size(&button);
            let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
            let response = ui.button(("bisect", label), cell, &button);
            if let Some(tooltip) = tooltip {
                ui.tooltip(&response, tooltip);
            }
            if response.clicked {
                intents.push(intent);
            }
        }
        let said = match &says {
            BannerSays::MarkBad => words::MARK_BAD_TO_BEGIN.to_owned(),
            BannerSays::MarkGood => words::MARK_GOOD_TO_BEGIN.to_owned(),
            BannerSays::Test(said) => said.clone(),
            BannerSays::Found(commit) => words::found(commit.short()),
        };
        let _ = ui.label(parts.rest(), &Label::new(&said));
    });
    intents
}

/// The buttons of the banner, from the right: **End**, then **Good**, **Bad** and **Skip** while commits are left to
/// test and no operation runs, or **Show** once the first bad commit is found.
fn buttons(says: &BannerSays, is_running: bool) -> Vec<(&'static str, Option<&'static str>, Intent)> {
    let mut buttons = vec![(
        words::END,
        Some(words::END_TOOLTIP),
        run(FeatureId("bisect.end"), Answers::new()),
    )];
    match says {
        BannerSays::Test(_) if !is_running => {
            for (label, tooltip, word) in [
                (words::SKIP, words::SKIP_TOOLTIP, "skip"),
                (words::BAD, words::BAD_TOOLTIP, "bad"),
                (words::GOOD, words::GOOD_TOOLTIP, "good"),
            ] {
                let answers = Answers::new().with(ANSWER, Answer::Choice(word.to_owned()));
                buttons.push((label, Some(tooltip), run(FeatureId("bisect.answer"), answers)));
            }
        }
        BannerSays::Found(commit) => {
            buttons.push((words::SHOW, None, Intent::Graph(GraphIntent::GoTo(commit.clone()))));
        }
        BannerSays::Test(_) | BannerSays::MarkBad | BannerSays::MarkGood => {}
    }
    buttons
}

/// The intent that runs a feature of the bisect on what is checked out, with the answers of its button.
fn run(feature: FeatureId, answers: Answers) -> Intent {
    if answers == Answers::new() {
        Intent::Feature(FeatureIntent::Run(feature, Target::Head))
    } else {
        Intent::Feature(FeatureIntent::RunWith(feature, Target::Head, answers))
    }
}

/// The icons and the tooltip of the label of a commit a bisect marked, drawn after its other labels like a tag's, its
/// words `bad`, `good` or `skipped`. A double click on it opens the commit's page, as on a tag, and it names no line of
/// the graph.
///
/// Feature: bisect.marks
#[must_use]
pub fn marks_view(mark: BisectMark) -> (&'static [Icon], Option<String>) {
    let icons: &'static [Icon] = match mark {
        BisectMark::Good => &[Icon::Check],
        BisectMark::Bad => &[Icon::Error],
        BisectMark::Skipped => &[Icon::Minus],
    };
    (icons, None)
}
