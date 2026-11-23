//! The notice of git's commit-graph file, in the status bar (`upkeep.offer-commit-graph`): for a repository with a long
//! history and no such file, that git can read it about twice as fast with one, with **Write it** and **Not for this
//! repository**.

use model::contract::{FeatureId, Target};
use ui::layout::Area;
use ui::{AccessRole, Button, ButtonKind, Description, Label, Sense, TextKind, Tone, Ui};

use super::said::link;
use crate::feature::FeatureIntent;
use crate::icons;
use crate::intent::Intent;
use crate::text::feature as feature_words;
use crate::text::repo as words;

/// The notice of the commit-graph file of the repository shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommitGraphNotice<'a> {
    /// The name of the repository, as the notice says it.
    pub repository: &'a str,
}

/// The notice of the commit-graph file, over the rectangle its caller clipped it to in the status bar: the icon of
/// information, the sentence cut with `…` where the room ends and whole in its tooltip, then **Write it**, which runs
/// `upkeep.write-commit-graph` and puts the notice away, **Not for this repository**, which says it no more for the
/// repository, and **×**, which puts it away until the repository is opened again. It stays until one is clicked.
///
/// Feature: upkeep.offer-commit-graph
pub fn commit_graph_notice(state: &CommitGraphNotice<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let mut intents = Vec::new();
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.medium);
    ui.icon_in(parts.left(theme.sizes.icon), icons::INFORMATION, Tone::Accent);
    let close = Button::icon(icons::CLOSE, feature_words::DISMISS);
    if link(ui, &mut parts, ("commit graph dismiss", &close)) {
        intents.push(Intent::DismissCommitGraph);
    }
    let decline = Button::secondary(words::NOT_FOR_THIS_REPOSITORY).kind(ButtonKind::Icon);
    if link(ui, &mut parts, ("commit graph decline", &decline)) {
        intents.push(Intent::DeclineCommitGraph);
    }
    let write = Button::secondary(words::WRITE_COMMIT_GRAPH).kind(ButtonKind::Icon);
    if link(ui, &mut parts, ("commit graph write", &write)) {
        let run = FeatureIntent::Run(FeatureId("upkeep.write-commit-graph"), Target::Head);
        intents.extend([Intent::Feature(run), Intent::DismissCommitGraph]);
    }
    let said = words::commit_graph_notice(state.repository);
    let text = parts.rest();
    let _ = ui.label(text, &Label::new(&said).kind(TextKind::Detail));
    let id = ui.id("commit graph words");
    let response = ui.interact(id, text, Sense::HOVER);
    ui.describe(id, text, &Description::new(AccessRole::Label, &said));
    ui.tooltip(&response, &said);
    intents
}
