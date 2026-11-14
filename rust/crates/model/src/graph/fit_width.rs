//! How wide the graph is drawn: as wide as its lines need, within limits, drawing its lines closer before it cuts any.

/// The room a lane has at its ease, in logical pixels.
const LANE_AT_EASE: f32 = 14.0;
/// The least room a lane is drawn in before lanes are cut off.
const LANE_AT_LEAST: f32 = 8.0;
/// The margin of the graph.
const MARGIN: f32 = 6.0;
/// How many lanes at their ease the graph takes at most while it chooses its own width.
const LANES_CHOSEN_AT_MOST: f32 = 16.0;
/// The part of the list the graph takes at most while it chooses its own width.
const CHOSEN_PART_OF_LIST: f32 = 1.0 / 3.0;
/// The part of the list the graph takes at most when it was dragged.
const DRAGGED_PART_OF_LIST: f32 = 0.6;

/// How the graph is drawn in the width it has.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraphFit {
    /// The width the graph is drawn in.
    pub width: f32,
    /// The room each lane gets in it.
    pub lane_width: f32,
    /// Whether lanes past its edge are cut off, and the graph fades out at its edge.
    pub is_cut: bool,
    /// The width at which every lane has its room at its ease.
    pub natural_width: f32,
}

/// How wide the graph is drawn, from the most lanes any row of the list has, the width the graph was dragged to (none
/// while it chooses its own), and the width of the list (0 while it is not known).
///
/// Choosing its own width, the graph takes its natural width, but no more than 16 lanes at their ease and no more than a
/// third of the list; dragged, it takes the width it was dragged to, but no more than its natural width and no more
/// than 60% of the list. It is never narrower than one lane at its ease. The lanes share the width, between 8 and 14
/// px each; when even 8 px each do not fit, the lanes past the edge are cut off.
///
/// Feature: graph.fit-width
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "a graph has a few hundred lanes at most, which an f32 holds exactly"
)]
pub fn fit_width(lanes: usize, wanted: Option<f32>, available: f32) -> GraphFit {
    let count = lanes as f32;
    let natural_width = count * LANE_AT_EASE + MARGIN;
    let list_limit = |part: f32| {
        if available > 0.0 {
            available * part
        } else {
            f32::INFINITY
        }
    };
    let width = match wanted {
        None => natural_width
            .min(LANES_CHOSEN_AT_MOST * LANE_AT_EASE + MARGIN)
            .min(list_limit(CHOSEN_PART_OF_LIST)),
        Some(wanted) => wanted.min(natural_width).min(list_limit(DRAGGED_PART_OF_LIST)),
    }
    .max(LANE_AT_EASE + MARGIN);
    if lanes == 0 {
        return GraphFit {
            width,
            lane_width: LANE_AT_EASE,
            is_cut: false,
            natural_width,
        };
    }
    let shared = (width - MARGIN) / count;
    GraphFit {
        width,
        lane_width: shared.clamp(LANE_AT_LEAST, LANE_AT_EASE),
        is_cut: shared < LANE_AT_LEAST,
        natural_width,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{GraphFit, fit_width};

    /// Whether a fit is the one expected, to a tenth of a pixel.
    fn assert_fit(fit: GraphFit, width: f32, lane_width: f32, is_cut: bool) {
        assert!((fit.width - width).abs() < 0.05, "{fit:?}");
        assert!((fit.lane_width - lane_width).abs() < 0.05, "{fit:?}");
        assert_eq!(fit.is_cut, is_cut, "{fit:?}");
    }

    #[test]
    fn graph_fit_width__three_lanes_take_their_natural_width() {
        assert_fit(fit_width(3, None, 1_000.0), 48.0, 14.0, false);
    }

    #[test]
    fn graph_fit_width__twenty_lanes_stop_at_sixteen_lanes_at_their_ease() {
        assert_fit(fit_width(20, None, 1_000.0), 230.0, 11.2, false);
    }

    #[test]
    fn graph_fit_width__forty_lanes_are_cut() {
        assert_fit(fit_width(40, None, 1_000.0), 230.0, 8.0, true);
    }

    #[test]
    fn graph_fit_width__twenty_lanes_take_no_more_than_a_third_of_the_list() {
        assert_fit(fit_width(20, None, 600.0), 200.0, 9.7, false);
    }

    #[test]
    fn graph_fit_width__dragged_wider_stops_at_the_natural_width() {
        let fit = fit_width(10, Some(400.0), 1_000.0);
        assert_fit(fit, 146.0, 14.0, false);
        assert!((fit.natural_width - 146.0).abs() < 0.05);
    }

    #[test]
    fn graph_fit_width__is_never_narrower_than_one_lane_and_has_no_list_limit_while_the_list_is_unknown() {
        assert_fit(fit_width(1, Some(5.0), 1_000.0), 20.0, 14.0, false);
        assert_fit(fit_width(0, None, 0.0), 20.0, 14.0, false);
        assert_fit(fit_width(10, None, 0.0), 146.0, 14.0, false);
        assert_fit(fit_width(40, Some(300.0), 400.0), 240.0, 8.0, true);
    }
}
