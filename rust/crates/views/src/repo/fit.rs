//! How the tabs fit the width of the island (`repo.tabs`): each as wide as its name, all of them narrower when they do
//! not fit, down to [`TAB_LEAST`], and those that still do not fit listed in **… N**.

use crate::number::{count_of_float, float_of_count};

/// The narrowest a tab gets before the tabs that do not fit are listed in **… N**, in logical pixels.
pub const TAB_LEAST: f32 = 96.0;

/// Where the tabs go.
#[derive(Clone, Debug, PartialEq)]
pub struct Fit {
    /// The tabs drawn, in their order, each by its place with its width.
    pub drawn: Vec<(usize, f32)>,
    /// The tabs listed in **… N**, by their places, in their order.
    pub listed: Vec<usize>,
}

/// Fits tabs as wide as their names in the room of the island, with a gap between them: all of them when they fit,
/// the widest narrowed first, none narrower than [`TAB_LEAST`] unless its name is; otherwise as many as fit at that
/// width beside **… N**, which is `more` wide, and the others listed in it. The tab shown is never listed: it takes the
/// place of the last tab drawn.
#[must_use]
pub fn fit_tabs(widths: &[f32], shown: Option<usize>, (room, gap, more): (f32, f32, f32)) -> Fit {
    let all: Vec<usize> = (0..widths.len()).collect();
    if let Some(widest) = widest(widths, &all, (room, gap)) {
        return Fit {
            drawn: all.iter().map(|&at| (at, width_of(widths, at).min(widest))).collect(),
            listed: Vec::new(),
        };
    }
    let space = room - more - gap;
    let count = count_of_float(((space + gap) / (TAB_LEAST + gap)).floor()).clamp(1, widths.len());
    let mut drawn: Vec<usize> = (0..count).collect();
    if let Some(shown) = shown.filter(|shown| *shown >= count && *shown < widths.len())
        && let Some(last) = drawn.last_mut()
    {
        *last = shown;
    }
    let widest = widest(widths, &drawn, (space, gap)).unwrap_or(TAB_LEAST);
    Fit {
        listed: all.into_iter().filter(|at| !drawn.contains(at)).collect(),
        drawn: drawn
            .into_iter()
            .map(|at| (at, width_of(widths, at).min(widest)))
            .collect(),
    }
}

/// The width of a tab, by its place.
fn width_of(widths: &[f32], at: usize) -> f32 {
    widths.get(at).copied().unwrap_or(TAB_LEAST)
}

/// The width no tab of these is wider than so that they fit the room, the narrower ones keeping theirs; none when they
/// do not fit even at [`TAB_LEAST`].
fn widest(widths: &[f32], tabs: &[usize], (room, gap): (f32, f32)) -> Option<f32> {
    let count = tabs.len();
    if count == 0 {
        return Some(f32::INFINITY);
    }
    let space = room - gap * float_of_count(count - 1);
    let mut sorted: Vec<f32> = tabs.iter().map(|&at| width_of(widths, at)).collect();
    sorted.sort_by(f32::total_cmp);
    if sorted.iter().sum::<f32>() <= space {
        return Some(f32::INFINITY);
    }
    let mut used = 0.0;
    for (index, width) in sorted.iter().enumerate() {
        let share = (space - used) / float_of_count(count - index);
        if share <= *width {
            return (share >= TAB_LEAST).then_some(share);
        }
        used += width;
    }
    None
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{Fit, TAB_LEAST, fit_tabs};

    #[test]
    fn repo_tabs__tabs_that_fit_keep_the_widths_of_their_names() {
        let fit = fit_tabs(&[100.0, 150.0], Some(0), (400.0, 4.0, 40.0));
        assert_eq!(
            fit,
            Fit {
                drawn: vec![(0, 100.0), (1, 150.0)],
                listed: Vec::new()
            }
        );
    }

    #[test]
    fn repo_tabs__tabs_that_do_not_fit_get_narrower_the_widest_first() {
        let fit = fit_tabs(&[100.0, 200.0, 200.0], None, (404.0, 2.0, 40.0));
        assert_eq!(fit.drawn, [(0, 100.0), (1, 150.0), (2, 150.0)]);
        assert_eq!(fit.listed, []);
    }

    #[test]
    fn repo_tabs__with_eight_tabs_in_a_narrow_island_the_others_are_listed_and_never_the_tab_shown() {
        let widths = [150.0; 8];
        let fit = fit_tabs(&widths, Some(6), (700.0, 4.0, 40.0));
        let drawn: Vec<usize> = fit.drawn.iter().map(|(at, _)| *at).collect();
        assert_eq!(
            drawn,
            [0, 1, 2, 3, 4, 6],
            "the tab shown takes the place of the last that fits"
        );
        assert_eq!(fit.listed, [5, 7]);
        assert!(fit.drawn.iter().all(|(_, width)| *width >= TAB_LEAST));
    }
}
