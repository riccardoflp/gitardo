//! Rules R3.1 and R3.2: the pure crates use no files, processes, network, threads, environment or clock; and no
//! crate but `app` and `platform` reads the time of day.

use super::Workspace;
use crate::problem::Problem;
use crate::source::code_only;

/// The crates that are pure functions of what they are given.
const PURE: [&str; 4] = ["crates/model", "crates/text", "crates/features", "crates/json"];

/// The modules of the standard library that reach outside the program.
const OUTSIDE: [&str; 5] = ["fs", "process", "net", "thread", "env"];

/// The crates that may read the time of day.
const CLOCK_READERS: [&str; 2] = ["crates/app", "crates/platform"];

/// The clocks no crate but the readers reads: the time of day.
const TIME_OF_DAY: &[&str] = &["SystemTime::now"];

/// The clocks a pure crate does not read: not even the monotonic one.
const EVERY_CLOCK: &[&str] = &["SystemTime::now", "Instant::now"];

/// The problems of purity in the crates of the workspace.
pub fn check(workspace: &Workspace) -> Vec<Problem> {
    let mut found = Vec::new();
    for member in workspace
        .members
        .iter()
        .filter(|member| member.folder.starts_with("crates/"))
    {
        let is_pure = PURE.contains(&member.folder.as_str());
        let reads_clock = CLOCK_READERS.contains(&member.folder.as_str());
        for (file, text) in workspace.rust_files(&member.folder) {
            let code = code_only(text);
            if is_pure {
                found.extend(outside_uses(&code).into_iter().map(|(line, what)| {
                    let message = format!("{} is pure and may not use {what}.", member.name);
                    Problem::at_line("R3.1", &file.relative, line, message)
                }));
            }
            if !reads_clock {
                let clocks = if is_pure { EVERY_CLOCK } else { TIME_OF_DAY };
                found.extend(clock_reads(&code, clocks).into_iter().map(|(line, what)| {
                    let message = format!("{what} is read only in app and platform; take the time as a value.");
                    Problem::at_line("R3.2", &file.relative, line, message)
                }));
            }
        }
    }
    found
}

/// The uses of the modules of [`OUTSIDE`] in some code, with their lines: `std::fs::read`, `use std::{env, fmt}`.
pub fn outside_uses(code: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (index, line) in code.lines().enumerate() {
        let mut rest = line;
        while let Some(start) = rest.find("std::") {
            let is_word_start = rest
                .get(..start)
                .and_then(|before| before.chars().last())
                .is_none_or(|previous| !(previous.is_alphanumeric() || previous == '_'));
            let after = rest.get(start + "std::".len()..).unwrap_or_default();
            if is_word_start {
                found.extend(
                    first_segments(after)
                        .into_iter()
                        .filter(|segment| OUTSIDE.contains(&segment.as_str()))
                        .map(|segment| (index + 1, format!("std::{segment}"))),
                );
            }
            rest = after;
        }
    }
    found
}

/// The first segments of the paths after `std::`: `fs::read` gives `fs`, `{env, fmt::Write}` gives `env` and `fmt`.
fn first_segments(after: &str) -> Vec<String> {
    let segment = |text: &str| text.trim().split("::").next().unwrap_or_default().trim().to_owned();
    if let Some(group) = after.strip_prefix('{') {
        return group
            .split('}')
            .next()
            .unwrap_or_default()
            .split(',')
            .map(segment)
            .collect();
    }
    let word = after
        .chars()
        .take_while(|character| character.is_alphanumeric() || *character == '_')
        .collect();
    vec![word]
}

/// The reads of some clocks in some code, with their lines.
fn clock_reads(code: &str, clocks: &[&'static str]) -> Vec<(usize, &'static str)> {
    let mut found = Vec::new();
    for (index, line) in code.lines().enumerate() {
        found.extend(
            clocks
                .iter()
                .filter(|clock| line.contains(**clock))
                .map(|clock| (index + 1, *clock)),
        );
    }
    found
}

#[cfg(test)]
mod tests {
    use super::{EVERY_CLOCK, TIME_OF_DAY, clock_reads, outside_uses};

    #[test]
    fn finds_the_modules_that_reach_outside() {
        let code = "use std::fs;\nuse std::{collections::BTreeMap, env, process::Command};\nlet a = ::std::thread::spawn;\n\
            use std::fmt;\nlet b = mystd::fs::x;\n";
        let found: Vec<(usize, String)> = outside_uses(code);
        let expected = [(1, "std::fs"), (2, "std::env"), (2, "std::process"), (3, "std::thread")];
        let expected: Vec<(usize, String)> = expected
            .iter()
            .map(|(line, what)| (*line, (*what).to_owned()))
            .collect();
        assert_eq!(found, expected);
    }

    #[test]
    fn finds_the_reads_of_the_clock() {
        let code = "let a = SystemTime::now();\nlet b = Instant::now();\n";
        assert_eq!(clock_reads(code, TIME_OF_DAY), vec![(1, "SystemTime::now")]);
        assert_eq!(
            clock_reads(code, EVERY_CLOCK),
            vec![(1, "SystemTime::now"), (2, "Instant::now")]
        );
    }
}
