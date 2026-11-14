//! The regions of a frame drawn since it was last shown, so that the window is sent only those.

use crate::geometry::PixelRect;

/// The most regions kept apart: past this, the two whose union wastes the fewest pixels become one.
const MOST_REGIONS: usize = 16;

/// How many pixels a union of two regions may add that neither held, for every pixel the two hold, before the two are
/// kept apart: sending a few pixels again costs less than sending another rectangle.
const WASTE_ALLOWED: u64 = 4;

/// The rectangles of a frame drawn since the last time they were taken, merged as they come: a region inside another
/// is dropped, two regions that touch or nearly fill their union become one, and there are never more than
/// [`MOST_REGIONS`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct DirtyRegions {
    /// The regions, none inside another.
    regions: Vec<PixelRect>,
}

impl DirtyRegions {
    /// Adds a region that was drawn.
    pub(crate) fn add(&mut self, region: PixelRect) {
        if region.is_empty() || self.regions.iter().any(|kept| kept.contains(region)) {
            return;
        }
        let mut merged = region;
        while let Some(position) = self.regions.iter().position(|kept| is_worth_merging(*kept, merged)) {
            merged = merged.union(self.regions.swap_remove(position));
        }
        self.regions.push(merged);
        if self.regions.len() > MOST_REGIONS {
            self.merge_cheapest_pair();
        }
    }

    /// The regions drawn since the last call, leaving none.
    pub(crate) fn take(&mut self) -> Vec<PixelRect> {
        let mut regions = std::mem::take(&mut self.regions);
        regions.sort_by_key(|region| (region.top, region.left));
        regions
    }

    /// Whether nothing was drawn since the regions were last taken.
    pub(crate) fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// Merges the two regions whose union adds the fewest pixels that neither holds.
    fn merge_cheapest_pair(&mut self) {
        let mut cheapest: Option<(u64, usize, usize)> = None;
        for (first_index, first) in self.regions.iter().enumerate() {
            for (second_index, second) in self.regions.iter().enumerate().skip(first_index + 1) {
                let cost = waste(*first, *second);
                if cheapest.is_none_or(|(least, _, _)| cost < least) {
                    cheapest = Some((cost, first_index, second_index));
                }
            }
        }
        if let Some((_, first_index, second_index)) = cheapest {
            let second = self.regions.swap_remove(second_index);
            if let Some(first) = self.regions.get_mut(first_index) {
                *first = first.union(second);
            }
        }
    }
}

/// The pixels a union of two regions holds that neither of them does (counting their overlap twice, which only makes
/// overlapping regions cheaper to merge).
fn waste(first: PixelRect, second: PixelRect) -> u64 {
    first.union(second).area().saturating_sub(first.area() + second.area())
}

/// Whether two regions are better sent as one: they overlap or touch with little waste, or their union is nearly
/// what they hold anyway.
fn is_worth_merging(first: PixelRect, second: PixelRect) -> bool {
    let held = first.area() + second.area();
    let wasted = waste(first, second);
    (first.touches(second) && wasted <= held) || wasted * WASTE_ALLOWED <= held
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_inside_others_are_dropped_and_neighbours_merged() {
        let mut regions = DirtyRegions::default();
        regions.add(PixelRect::new(0, 0, 100, 20));
        regions.add(PixelRect::new(10, 5, 20, 10));
        regions.add(PixelRect::new(0, 20, 100, 40));
        assert_eq!(regions.take(), vec![PixelRect::new(0, 0, 100, 40)]);
        assert!(regions.is_empty());
    }

    #[test]
    fn regions_far_apart_stay_apart() {
        let mut regions = DirtyRegions::default();
        regions.add(PixelRect::new(0, 0, 10, 10));
        regions.add(PixelRect::new(500, 500, 510, 510));
        assert_eq!(regions.take().len(), 2);
    }

    #[test]
    fn there_are_never_more_regions_than_the_limit() {
        let mut regions = DirtyRegions::default();
        for index in 0..100 {
            regions.add(PixelRect::new(index * 50, index * 50, index * 50 + 5, index * 50 + 5));
        }
        let taken = regions.take();
        assert!(taken.len() <= MOST_REGIONS);
        for index in 0..100 {
            let region = PixelRect::new(index * 50, index * 50, index * 50 + 5, index * 50 + 5);
            assert!(taken.iter().any(|kept| kept.contains(region)), "{region:?} is lost");
        }
    }
}
