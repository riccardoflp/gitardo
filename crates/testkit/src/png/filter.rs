//! The filters of PNG rows: each row is written as the difference from a prediction made from the bytes before it,
//! and read back by adding the prediction again.
//!
//! The PNG specification, "Portable Network Graphics (PNG) Specification (Second Edition)", 2003, section 9.

use crate::error::Error;

/// The five filter types of filter method 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Filter {
    /// The bytes are as they are.
    None = 0,
    /// The difference from the byte of the pixel on the left.
    Sub = 1,
    /// The difference from the byte of the pixel above.
    Up = 2,
    /// The difference from the average of the pixels on the left and above.
    Average = 3,
    /// The difference from whichever of the pixels on the left, above and above on the left is nearest to their
    /// gradient, Paeth's predictor.
    Paeth = 4,
}

impl Filter {
    /// The filter of a filter type byte, or nothing when there is no such type.
    pub(crate) fn from_byte(byte: u8) -> Option<Filter> {
        match byte {
            0 => Some(Filter::None),
            1 => Some(Filter::Sub),
            2 => Some(Filter::Up),
            3 => Some(Filter::Average),
            4 => Some(Filter::Paeth),
            _ => None,
        }
    }

    /// What the filter predicts a byte to be, from the bytes of the same channel on the left, above, and above on the
    /// left: 0 where there is no such pixel.
    fn prediction(self, neighbours: Neighbours) -> u8 {
        let Neighbours {
            left,
            above,
            above_left,
        } = neighbours;
        match self {
            Filter::None => 0,
            Filter::Sub => left,
            Filter::Up => above,
            // The midpoint of two unsigned numbers is rounded down, as the filter calls for.
            Filter::Average => u8::midpoint(left, above),
            Filter::Paeth => paeth(left, above, above_left),
        }
    }
}

/// The bytes of the same channel in the pixels next to a byte that were read before it.
#[derive(Clone, Copy, Debug)]
struct Neighbours {
    left: u8,
    above: u8,
    above_left: u8,
}

/// Paeth's predictor: of the three neighbours, the one nearest to left + above − above left, the left first and the
/// one above second when two are as near.
fn paeth(left: u8, above: u8, above_left: u8) -> u8 {
    let (left_value, above_value, above_left_value) = (i16::from(left), i16::from(above), i16::from(above_left));
    let estimate = left_value + above_value - above_left_value;
    let to_left = (estimate - left_value).abs();
    let to_above = (estimate - above_value).abs();
    let to_above_left = (estimate - above_left_value).abs();
    if to_left <= to_above && to_left <= to_above_left {
        left
    } else if to_above <= to_above_left {
        above
    } else {
        above_left
    }
}

/// The bytes of the rows of an image, from the decompressed data where each row is its filter type and its filtered
/// bytes. The data must hold whole rows.
pub(crate) fn unfilter(data: &[u8], row_length: usize, bytes_per_pixel: usize) -> Result<Vec<u8>, Error> {
    let mut rows = Vec::with_capacity(data.len());
    let mut previous: Vec<u8> = Vec::new();
    for (row, filtered) in data.chunks_exact(row_length + 1).enumerate() {
        let Some((filter_type, bytes)) = filtered.split_first() else {
            continue;
        };
        let filter = Filter::from_byte(*filter_type).ok_or(Error::UnknownFilter {
            row,
            filter: *filter_type,
        })?;
        let mut current = bytes.to_vec();
        for index in 0..current.len() {
            let neighbours = neighbours(&current, &previous, index, bytes_per_pixel);
            if let Some(byte) = current.get_mut(index) {
                *byte = byte.wrapping_add(filter.prediction(neighbours));
            }
        }
        rows.extend_from_slice(&current);
        previous = current;
    }
    Ok(rows)
}

/// The neighbours of a byte of a row, in the row and in the row above it, which is empty for the first row.
fn neighbours(row: &[u8], previous: &[u8], index: usize, bytes_per_pixel: usize) -> Neighbours {
    let left_index = index.checked_sub(bytes_per_pixel);
    Neighbours {
        left: left_index.and_then(|left| row.get(left)).copied().unwrap_or(0),
        above: previous.get(index).copied().unwrap_or(0),
        above_left: left_index.and_then(|left| previous.get(left)).copied().unwrap_or(0),
    }
}

/// A row filtered as a writer would filter it, for the tests of the reader.
#[cfg(test)]
pub(crate) fn filter_row(filter: Filter, row: &[u8], previous: &[u8], bytes_per_pixel: usize) -> Vec<u8> {
    (0..row.len())
        .map(|index| {
            let prediction = filter.prediction(neighbours(row, previous, index, bytes_per_pixel));
            row[index].wrapping_sub(prediction)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paeth_picks_the_neighbour_nearest_to_the_gradient() {
        assert_eq!(paeth(10, 20, 10), 20);
        assert_eq!(paeth(20, 10, 10), 20);
        assert_eq!(paeth(10, 10, 10), 10);
        assert_eq!(paeth(0, 255, 255), 0);
        assert_eq!(paeth(100, 50, 200), 50);
        assert_eq!(paeth(50, 100, 60), 100);
    }

    #[test]
    fn every_filter_undoes_what_it_did() {
        let rows: [&[u8]; 3] = [
            &[1, 2, 3, 250, 251, 252],
            &[9, 8, 7, 6, 5, 4],
            &[255, 0, 128, 3, 200, 100],
        ];
        for filter in [Filter::None, Filter::Sub, Filter::Up, Filter::Average, Filter::Paeth] {
            let mut data = Vec::new();
            let mut previous: &[u8] = &[];
            for row in rows {
                data.push(filter as u8);
                data.extend(filter_row(filter, row, previous, 3));
                previous = row;
            }
            assert_eq!(unfilter(&data, 6, 3).unwrap(), rows.concat(), "{filter:?}");
        }
    }

    #[test]
    fn an_unknown_filter_type_names_its_row() {
        let data = [0, 1, 2, 5, 1, 2];
        assert!(matches!(
            unfilter(&data, 2, 1),
            Err(Error::UnknownFilter { row: 1, filter: 5 })
        ));
    }
}
