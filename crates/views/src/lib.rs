//! Every place and page of the window of Gitardo Desktop, as functions of state that return what the user wants.
//!
//! A view has the signature `fn(state: &State, ui: &mut Ui) -> Vec<Intent>` (rule R2.4): it draws its state and says
//! what the user asked for, and changes nothing. Each place draws over the rectangle its caller clipped it to
//! (`ui.current_clip()`), so that the window lays out the places and each place lays out itself. The state of each
//! view is its own type here, made by the application from its store for each frame; the [`Intent`]s go back to the
//! store, which reduces them.
//!
//! The places of [01-product.md](../../docs/01-product.md): the header ([`graph::toolbar_view`]), the branch rail
//! ([`refs::tree_view`]), the timeline ([`graph::view`]), the inspector ([`details::show`]), the pages with their trail
//! ([`page::host_view`]), the Activity drawer ([`activity::what_is_running_view`]), the search box at the top of the
//! timeline ([`search::box_view`]) and Quick search ([`search::quick_search_view`]); and [`view`], the whole window. All the words are in [`text`] (rule R10.2).

#![forbid(unsafe_code)]

pub mod activity;
pub mod bisect;
mod blocks;
pub mod branch;
pub mod changes;
pub mod commit_page;
pub mod compare;
pub mod conflicts;
pub mod details;
pub mod diff;
pub mod feature;
pub mod file_view;
pub mod filter;
pub mod graph;
mod intent;
mod line;
pub mod log;
pub mod merge_view;
mod number;
pub mod page;
pub mod patch;
pub mod rebase;
pub mod refs;
pub mod repo;
pub mod review;
pub mod search;
mod selection;
pub mod settings;
pub mod text;
pub mod welcome;
mod window;

pub use intent::{FilterIntent, FolderAnswer, Intent, Notice, RailIntent, RailList, RailLists};
pub use window::{Content, FeatureDialog, TimelineState, WindowState, view};
