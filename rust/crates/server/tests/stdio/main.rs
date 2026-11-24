//! `gitardo-engine --stdio` run as the extension runs it: messages framed on its standard input, answers and
//! notifications read from its standard output, against throwaway repositories. These are the acceptance lines of
//! stages 1, 2, 5 and 7 of `docs/transition/06-roadmap.md` that the engine alone can show.

#![cfg(test)]

mod ai;
mod feature;
mod harness;
mod help;
mod hosting;
mod journal;
mod mermaid;
mod range_diff;
mod reads;
mod reads_commit_box;
mod reads_graph;
mod reads_lfs;
mod reads_repository;
mod reads_tags;
mod repository;
mod summaries;
