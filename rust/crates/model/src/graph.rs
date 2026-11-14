//! The graph of commits: its rows, the lines between them, how wide it is drawn, the labels of each commit, the history
//! that stands out, the lines between days and months, the merges folded, the branch each line comes from, and the
//! stashes among the commits.

mod combine_upstream;
mod date_separators;
mod fit_width;
mod fold_rows;
mod labels;
mod lane_tips;
mod lanes;
mod page_size;
mod place_stashes;
mod reach;
mod row;
#[cfg(test)]
pub(crate) mod testing;
mod trace_history;
mod unpushed;

pub use combine_upstream::combine_upstream;
pub(crate) use date_separators::civil_date;
pub use date_separators::{DateTime, date_separators};
pub use fit_width::{GraphFit, fit_width};
pub use fold_rows::{Fold, FoldedRows, ShownRow, fold_rows, foldable_merges};
pub use labels::{BisectMark, GraphSettings, Label, LabelKind, labels};
pub use lane_tips::lane_tips;
pub use lanes::{CellLine, GraphCell, Half, PALETTE_SIZE, lanes};
pub use page_size::PageSize;
pub use place_stashes::{StashCommit, place_stashes};
pub use row::{GraphInput, Row};
pub use trace_history::{CellTrace, Trace, trace_history};
pub use unpushed::unpushed;
