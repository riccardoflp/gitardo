//! The edge between the graph and the messages (`graph.resize-graph`): dragged, it sets the width the graph is drawn
//! in, from 20 px to the width where every lane has its room and no more than 60% of the list; a double click lets the
//! graph choose again.

use model::graph::{GraphFit, fit_width};
use ui::{Split, Splitter, Ui};

use super::intent::GraphIntent;
use crate::intent::Intent;
use crate::text::timeline as words;

/// The least width the graph is dragged to.
pub const LEAST: f32 = 20.0;
/// The most of the list the graph is dragged to.
pub const MOST_OF_LIST: f32 = 0.6;
/// How far an arrow moves the edge.
pub const STEP: f32 = 16.0;

/// The graph and its edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphEdgeState {
    /// How many lanes the graph has.
    pub lanes: usize,
    /// The width it was dragged to; none while it chooses its own.
    pub wanted: Option<f32>,
}

impl GraphEdgeState {
    /// The width the graph is drawn in, in a list of a width.
    #[must_use]
    pub fn fit(&self, available: f32) -> GraphFit {
        fit_width(self.lanes, self.wanted, available)
    }
}

/// The edge over the list its caller clipped it to, which stays where it is while the commits scroll.
///
/// Feature: graph.resize-graph
pub fn resize_graph_view(state: &GraphEdgeState, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let fit = state.fit(rect.width);
    let most = fit.natural_width.min(rect.width * MOST_OF_LIST).max(LEAST);
    let line = ui.theme().sizes.border;
    let splitter = Splitter {
        split: Split::Columns,
        default_size: fit_width(state.lanes, None, rect.width).width,
        least_first: LEAST,
        least_second: (rect.width - line - most).max(0.0),
        size: Some(fit.width),
        step: STEP,
    };
    let response = ui.scope(words::GRAPH_EDGE, |ui| ui.splitter("graph edge", rect, &splitter));
    if response.is_reset {
        vec![Intent::Graph(GraphIntent::SetGraphWidth(None))]
    } else if response.is_changed {
        vec![Intent::Graph(GraphIntent::SetGraphWidth(Some(response.size)))]
    } else {
        Vec::new()
    }
}
