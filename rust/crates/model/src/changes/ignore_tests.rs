#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::PathBuf;

use super::{IgnoreChoices, IgnoreInput, ignore_patterns, with_line};

fn choices(path: &str, is_folder: bool) -> IgnoreChoices {
    ignore_patterns(&IgnoreInput {
        path: path.to_owned(),
        is_folder,
        exclude: PathBuf::from("/repo/.git/info/exclude"),
    })
}

/// The lines of each pattern in a place, by its place in the list.
fn lines_in(choices: &IgnoreChoices, place: usize) -> Vec<Option<String>> {
    let place = &choices.places[place];
    choices.patterns.iter().map(|pattern| pattern.line_in(place)).collect()
}

#[test]
fn changes_ignore_patterns__a_log_file_in_logs_gives_this_file_its_extension_and_its_folder() {
    let choices = choices("logs/today.log", false);
    let words: Vec<&str> = choices.patterns.iter().map(|pattern| pattern.words.as_str()).collect();
    assert_eq!(words, ["This file", "All .log files", "The folder logs"]);
    let top: Vec<String> = lines_in(&choices, 0).into_iter().flatten().collect();
    assert_eq!(top, ["/logs/today.log", "*.log", "/logs/"]);
    assert_eq!(choices.places[1].file, PathBuf::from("logs/.gitignore"));
    assert_eq!(
        lines_in(&choices, 1),
        [Some(String::from("/today.log")), Some(String::from("*.log")), None]
    );
    assert!(choices.places[2].is_exclude);
}

#[test]
fn changes_ignore_patterns__a_dot_file_at_the_top_gives_this_file_alone_and_a_tar_gz_its_last_extension() {
    let env = choices(".env", false);
    assert_eq!(env.patterns.len(), 1);
    assert_eq!(env.patterns[0].shown, "/.env");
    assert_eq!(env.places.len(), 2, "the repository's .gitignore and the exclude file");
    let archive = choices("dist/app.tar.gz", false);
    assert!(archive.patterns.iter().any(|pattern| pattern.shown == "*.gz"));
}

#[test]
fn changes_ignore_patterns__a_folder_gives_this_folder_and_every_folder_of_its_name() {
    let choices = choices("packages/web/node_modules", true);
    let shown: Vec<&str> = choices.patterns.iter().map(|pattern| pattern.shown.as_str()).collect();
    assert_eq!(shown, ["/packages/web/node_modules/", "node_modules/"]);
}

#[test]
fn changes_ignore_patterns__a_name_with_a_bracket_is_written_with_a_backslash_before_it() {
    let choices = choices("notes[1].txt", false);
    assert_eq!(lines_in(&choices, 0)[0].as_deref(), Some("/notes\\[1\\].txt"));
}

#[test]
fn changes_ignore_patterns__the_line_goes_on_a_line_of_its_own_with_the_line_ending_of_the_file() {
    assert_eq!(with_line(Some("*.tmp"), "*.log").as_deref(), Some("*.tmp\n*.log\n"));
    assert_eq!(
        with_line(Some("*.tmp\r\nbuild/\r\n"), "*.log").as_deref(),
        Some("*.tmp\r\nbuild/\r\n*.log\r\n")
    );
    assert_eq!(with_line(None, "*.log").as_deref(), Some("*.log\n"));
}

#[test]
fn changes_ignore_patterns__a_file_that_has_the_pattern_already_is_given_no_second_one() {
    assert_eq!(with_line(Some("*.log \nbuild/\n"), "*.log"), None);
}
