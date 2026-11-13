//! `conflicts.resolve-by-rules` against git: the worked examples of the card, and files made at random, merged by git
//! in a throwaway repository and resolved by the rules.

// The helpers of these tests fail as the tests do, loudly: clippy allows them what it allows tests.
#![cfg(test)]
#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

#[path = "conflict_rules/check.rs"]
mod check;
#[path = "conflict_rules/fuzz.rs"]
mod fuzz;
#[path = "conflict_rules/merges.rs"]
mod merges;
#[path = "conflict_rules/numbered.rs"]
mod numbered;
#[path = "conflict_rules/repeating.rs"]
mod repeating;
#[path = "conflict_rules/table.rs"]
mod table;
