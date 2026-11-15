//! `gitardo-engine --stdio` run as the extension runs it: messages framed on its standard input, answers and
//! notifications read from its standard output, against throwaway repositories. These are the acceptance lines of
//! stages 1 and 2 of `docs/transition/06-roadmap.md` that the engine alone can show.

#![cfg(test)]

mod feature;
mod harness;
mod repository;
