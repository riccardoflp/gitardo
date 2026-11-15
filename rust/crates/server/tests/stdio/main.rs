//! `gitardo-engine --stdio` run as the extension runs it: messages framed on its standard input, answers and
//! notifications read from its standard output, against throwaway repositories. These are the acceptance lines of
//! stages 1, 2 and 5 of `docs/transition/06-roadmap.md` that the engine alone can show.

#![cfg(test)]

mod feature;
mod harness;
mod journal;
mod reads;
mod reads_repository;
mod repository;
