//! What new worktrees get from the checkout: what git ignores there with its size, and what the link and copy patterns
//! take.

mod ignored;
mod paths;
mod patterns;

pub use ignored::{IgnoredRequest, ignored_arguments, parse_ignored, read_ignored};
pub use paths::{
    committed_at_top_arguments, ignored_of_arguments, parse_ignored_of, parse_tracked, read_committed_at_top,
    read_path_status, tracked_arguments,
};
pub use patterns::{
    EmptyFolders, Listing, PatternsRequest, commit_files_arguments, files_of_arguments, ignored_files_arguments,
    ignored_folders_arguments, ignored_folders_input, ignored_whole_arguments, matched_arguments, parse_folders,
    parse_paths, read_what_patterns_take,
};
