#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::{ChooseInput, ChosenLine, GetsInput, ProposedLine, SetupPatterns, choose, line_gets, use_proposed};
use crate::worktree_setup::{Gets, SetupPreview};

fn strings(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|text| (*text).to_owned()).collect()
}

fn patterns(link: &[&str], copy: &[&str]) -> SetupPatterns {
    SetupPatterns {
        link: strings(link),
        copy: strings(copy),
    }
}

fn chosen(start: &SetupPatterns, line: ChosenLine<'_>, gets: Gets) -> SetupPatterns {
    choose(&ChooseInput {
        patterns: start,
        line,
        gets,
    })
}

#[test]
fn worktree_setup_choose__copy_moves_a_file_from_the_folders_to_link() {
    let start = patterns(&["/.env"], &[]);
    let line = ChosenLine::One {
        path: ".env",
        is_folder: false,
        pattern: "/.env",
    };
    assert_eq!(chosen(&start, line, Gets::Copy), patterns(&[], &["/.env"]));
}

#[test]
fn worktree_setup_choose__nothing_for_a_name_takes_out_the_name_and_each_place() {
    let places = strings(&["node_modules", "packages/api/node_modules", "packages/web/node_modules"]);
    let start = patterns(
        &["node_modules/", "/packages/api/node_modules/", "dist/"],
        &["/packages/web/node_modules/"],
    );
    let line = ChosenLine::Name {
        places: &places,
        is_folder: true,
    };
    assert_eq!(chosen(&start, line, Gets::Nothing), patterns(&["dist/"], &[]));
    assert_eq!(
        chosen(&start, line, Gets::Link),
        patterns(&["dist/", "node_modules/"], &[])
    );
}

#[test]
fn worktree_setup_choose__one_place_takes_its_path_and_the_other_keeps_what_the_name_gave() {
    let others = strings(&["a/.env.local"]);
    let start = patterns(&[".env.local"], &[]);
    let line = ChosenLine::Place {
        path: "b/.env.local",
        is_folder: false,
        others: &others,
    };
    assert_eq!(
        chosen(&start, line, Gets::Copy),
        patterns(&["/a/.env.local"], &["/b/.env.local"])
    );
    // Choosing for one place what the name gives them all only takes out the pattern of its path.
    let start = patterns(&[".env.local", "/b/.env.local"], &[]);
    assert_eq!(chosen(&start, line, Gets::Link), patterns(&[".env.local"], &[]));
}

#[test]
fn worktree_setup_choose__a_pattern_that_takes_more_stays() {
    let start = patterns(&[], &["*.local", ".env.local"]);
    let line = ChosenLine::One {
        path: ".env.local",
        is_folder: false,
        pattern: "/.env.local",
    };
    assert_eq!(chosen(&start, line, Gets::Nothing), patterns(&[], &["*.local"]));
}

#[test]
fn uses_what_is_proposed_by_name_and_by_path() {
    let places = strings(&["node_modules", "packages/api/node_modules"]);
    let lines = [
        ProposedLine {
            line: ChosenLine::Name {
                places: &places,
                is_folder: true,
            },
            proposed: Gets::Link,
        },
        ProposedLine {
            line: ChosenLine::One {
                path: ".env",
                is_folder: false,
                pattern: "/.env",
            },
            proposed: Gets::Copy,
        },
    ];
    assert_eq!(
        use_proposed(&SetupPatterns::default(), &lines),
        patterns(&["node_modules/"], &["/.env"])
    );
}

#[test]
fn says_what_a_line_gets_from_its_patterns_then_from_git() {
    let preview = SetupPreview {
        linked: strings(&["node_modules"]),
        copied: strings(&["config"]),
        ..SetupPreview::default()
    };
    let looked = patterns(&["*_modules/"], &["/config/", "/.env"]);
    let gets = |path: &str, is_folder: bool, now: &SetupPatterns| {
        line_gets(&GetsInput {
            path,
            is_folder,
            patterns: now,
            looked: &looked,
            preview: &preview,
        })
    };
    let now = patterns(&["*_modules/", "/dist/"], &["/.env"]);
    assert_eq!(gets("dist", true, &now), Gets::Link);
    assert_eq!(gets(".env", false, &now), Gets::Copy);
    assert_eq!(gets("node_modules", true, &now), Gets::Link);
    assert_eq!(
        gets("config", true, &now),
        Gets::Nothing,
        "its own pattern was taken out since"
    );
    assert_eq!(
        gets("config/app.json", false, &now),
        Gets::Copy,
        "git said its folder is copied"
    );
    assert_eq!(gets("notes", true, &now), Gets::Nothing);
}
