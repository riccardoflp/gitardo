//! `range-diff.read-last-rewrite` against throwaway repositories, and `range-diff.read-pairs` on what `git range-diff`
//! printed for rewrites recorded in `tests/golden/range-diff`.

use std::fmt::Write as _;
use std::io::Write as _;
use std::process::Stdio;

use git::read::range_diff::{
    LastRewriteRequest, RangeDiffRequest, parse_pairs, range_diff_arguments, read_last_rewrite, read_pairs,
};
use git::{Git, GitError};
use model::CommitId;
use model::range_diff::{Line, PairKind, Ranges, Rewrite, RewriteKind};
use testkit::{Remote, Repository};

use crate::golden::{Case, check, record};

/// The read of a repository with the configuration of the tests alone.
fn git_of(repository: &Repository) -> Git {
    let folder = repository.path().parent().unwrap();
    Git::new(repository.path())
        .with_variable("GIT_CONFIG_GLOBAL", folder.join("global.gitconfig"))
        .with_variable("GIT_CONFIG_NOSYSTEM", "1")
}

/// The last rewrite of a reference.
fn last_rewrite(repository: &Repository, reference: &str) -> Option<Rewrite> {
    let request = LastRewriteRequest {
        reference: reference.to_owned(),
    };
    read_last_rewrite(&git_of(repository), &request).unwrap()
}

/// The commit a revision names.
fn commit(repository: &Repository, revision: &str) -> CommitId {
    CommitId::parse(repository.git(&["rev-parse", revision]).trim()).unwrap()
}

/// `main` with the readme, then `feature` with two commits, then one more commit on `main`, with `feature` checked out.
fn diverged() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let _ = repository.git(&["checkout", "--quiet", "-b", "feature"]);
    let _ = repository.commit("Add the parser", &[("parser.rs", "parse\n")]);
    let _ = repository.commit("Add the lexer", &[("lexer.rs", "lex\n")]);
    let _ = repository.git(&["checkout", "--quiet", "main"]);
    let _ = repository.commit("Add the licence", &[("LICENCE", "MIT\n")]);
    let _ = repository.git(&["checkout", "--quiet", "feature"]);
    repository
}

#[test]
fn range_diff_read_last_rewrite__after_git_rebase_main_on_feature_it_gives_the_tips_and_the_commit_of_main() {
    let repository = diverged();
    let before = commit(&repository, "feature");
    let _ = repository.git(&["rebase", "--quiet", "main"]);
    assert_eq!(
        last_rewrite(&repository, "refs/heads/feature"),
        Some(Rewrite {
            kind: RewriteKind::Rebase {
                onto: commit(&repository, "main")
            },
            before,
            after: commit(&repository, "feature"),
        })
    );
}

#[test]
fn range_diff_read_last_rewrite__after_git_pull_rebase_it_gives_the_same_onto_the_upstream() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let _remote = Remote::new(&repository);
    let _ = repository.commit("Add the parser", &[("parser.rs", "parse\n")]);
    let _ = repository.git(&["push", "--quiet"]);
    let _ = repository.git(&["reset", "--quiet", "--hard", "HEAD~1"]);
    let before = repository.commit("Add the lexer", &[("lexer.rs", "lex\n")]);
    let _ = repository.git(&["pull", "--quiet", "--rebase"]);
    assert_eq!(
        last_rewrite(&repository, "refs/heads/main"),
        Some(Rewrite {
            kind: RewriteKind::Rebase {
                onto: commit(&repository, "origin/main")
            },
            before: CommitId::parse(&before).unwrap(),
            after: commit(&repository, "main"),
        })
    );
}

#[test]
fn range_diff_read_last_rewrite__after_an_amend_it_gives_the_commit_before_and_the_commit_after() {
    let mut repository = Repository::new();
    let first = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let _ = repository.git(&["commit", "--quiet", "--amend", "-m", "Add the readme of Gitardo"]);
    let root = last_rewrite(&repository, "refs/heads/main").unwrap();
    assert_eq!(
        root.kind,
        RewriteKind::Amend { is_root: true },
        "the first commit has no parent"
    );
    assert_eq!(root.before.as_str(), first);
    let before = repository.commit("Add the parser", &[("parser.rs", "parse\n")]);
    let _ = repository.git(&["commit", "--quiet", "--amend", "-m", "Add the parser of the queries"]);
    assert_eq!(
        last_rewrite(&repository, "refs/heads/main"),
        Some(Rewrite {
            kind: RewriteKind::Amend { is_root: false },
            before: CommitId::parse(&before).unwrap(),
            after: commit(&repository, "main"),
        })
    );
}

#[test]
fn range_diff_read_last_rewrite__after_a_commit_made_on_top_of_a_rebase_it_still_gives_the_rebase() {
    let mut repository = diverged();
    let _ = repository.git(&["rebase", "--quiet", "main"]);
    let rebased = commit(&repository, "feature");
    let _ = repository.commit("Add the tests", &[("tests.rs", "test\n")]);
    let rewrite = last_rewrite(&repository, "refs/heads/feature").unwrap();
    assert!(matches!(rewrite.kind, RewriteKind::Rebase { .. }));
    assert_eq!(rewrite.after, rebased);
}

#[test]
fn range_diff_read_last_rewrite__after_a_fetch_that_found_origin_main_force_pushed_it_gives_before_and_after() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let _ = repository.commit("Add the parser", &[("parser.rs", "parse\n")]);
    let remote = Remote::new(&repository);
    let before = commit(&repository, "origin/main");
    let _ = repository.git(&["checkout", "--quiet", "-b", "side", "HEAD~1"]);
    let after = repository.commit("Add the lexer", &[("lexer.rs", "lex\n")]);
    let _ = repository.git(&["push", "--quiet", "origin", "side"]);
    // Someone else force pushes `main` to the commit of `side`.
    let _ = remote.git(&["update-ref", "refs/heads/main", &after]);
    let _ = repository.git(&["fetch", "--quiet"]);
    assert_eq!(
        last_rewrite(&repository, "refs/remotes/origin/main"),
        Some(Rewrite {
            kind: RewriteKind::ForcePush,
            before,
            after: CommitId::parse(&after).unwrap(),
        })
    );
}

#[test]
fn range_diff_read_last_rewrite__a_branch_that_was_only_ever_committed_to_gives_nothing() {
    let repository = diverged();
    assert_eq!(last_rewrite(&repository, "refs/heads/feature"), None);
    assert_eq!(last_rewrite(&repository, "refs/heads/no-such-branch"), None);
}

/// `main` with the readme, `old` with four commits on it, and `feature` made again from them: the second reworded from
/// the first subject to the second, the fourth dropped, and with `added` a fifth one at the end.
fn rewritten((old, new): (&str, &str), added: bool) -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let _ = repository.git(&["checkout", "--quiet", "-b", "old"]);
    let _ = repository.commit("Add the parser", &[("parser.rs", "parse\n")]);
    let _ = repository.commit(old, &[("lexer.rs", "lex\n")]);
    let _ = repository.commit("Add the checker", &[("checker.rs", "check\n")]);
    let _ = repository.commit("Add the printer", &[("printer.rs", "print\n")]);
    let _ = repository.git(&["checkout", "--quiet", "-b", "feature", "main"]);
    let _ = repository.git(&["cherry-pick", "old~3"]);
    let _ = repository.git(&["cherry-pick", "old~2"]);
    let _ = repository.git(&["commit", "--quiet", "--amend", "-m", new]);
    let _ = repository.git(&["cherry-pick", "old~1"]);
    if added {
        let _ = repository.commit("Add the tests", &[("tests.rs", "test\n")]);
    }
    repository
}

/// The subject of the second commit before and after it was reworded.
const REWORDED: (&str, &str) = ("Add the lexer", "Add the lexer of the queries");

/// The request of `main old feature`.
fn request() -> RangeDiffRequest {
    let line = |to: &str| Line {
        from: String::from("main"),
        to: to.to_owned(),
    };
    RangeDiffRequest {
        ranges: Ranges {
            arguments: ["main", "old", "feature"].map(String::from).to_vec(),
            before: line("old"),
            now: line("feature"),
        },
    }
}

/// A case of the golden files of `git range-diff`.
fn case(name: &'static str) -> Case {
    Case {
        read: "range-diff",
        name,
    }
}

/// The kinds of the pairs of a case, in order.
fn kinds(name: &'static str) -> Vec<PairKind> {
    let output = std::fs::read(golden_path(name)).unwrap();
    parse_pairs(&output).unwrap().iter().map(|pair| pair.kind).collect()
}

/// The recorded output of a case.
fn golden_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/golden/range-diff/{name}.out"))
}

#[test]
fn range_diff_read_pairs__after_a_rebase_with_one_commit_reworded_and_one_dropped_it_gives_same_changed_and_dropped() {
    let reworded = case("reworded-and-dropped");
    record(
        reworded,
        || rewritten(REWORDED, false),
        &range_diff_arguments(&request()),
    );
    check(reworded, |output| parse_pairs(output).unwrap());
    assert_eq!(
        kinds("reworded-and-dropped"),
        [PairKind::Same, PairKind::Changed, PairKind::Same, PairKind::Dropped]
    );
    let output = std::fs::read(golden_path("reworded-and-dropped")).unwrap();
    let pairs = parse_pairs(&output).unwrap();
    assert!(pairs[1].diff.iter().any(|line| line == "  ## Commit message ##"));
    assert!(pairs[1].changed_parts().message && !pairs[1].changed_parts().changes);
    assert_eq!(pairs[3].now, None);
    assert_eq!(pairs[3].subject, "Add the printer");
    let read = read_pairs(&git_of(&rewritten(REWORDED, false)), &request()).unwrap();
    assert_eq!((read.before_count, read.now_count), (4, 3));
    assert_eq!(
        read.pairs[1].subject, REWORDED.1,
        "the subject now, which git does not print"
    );
}

#[test]
fn range_diff_read_pairs__a_commit_only_in_the_second_line_is_a_pair_added_with_no_commit_before() {
    let added = case("added");
    record(
        added,
        || rewritten(("Add the lexer", "Add the lexer"), true),
        &range_diff_arguments(&request()),
    );
    check(added, |output| parse_pairs(output).unwrap());
    let output = std::fs::read(golden_path("added")).unwrap();
    let pairs = parse_pairs(&output).unwrap();
    let added = pairs.iter().find(|pair| pair.kind == PairKind::Added).unwrap();
    assert_eq!((added.before.as_ref(), added.subject.as_str()), (None, "Add the tests"));
    assert!(added.now.is_some());
}

#[test]
fn range_diff_read_pairs__every_hash_is_given_in_full() {
    let output = std::fs::read(golden_path("reworded-and-dropped")).unwrap();
    for pair in parse_pairs(&output).unwrap() {
        for hash in pair.before.iter().chain(pair.now.iter()) {
            assert_eq!(hash.as_str().len(), 40, "{hash}");
        }
    }
}

#[test]
fn range_diff_read_pairs__a_recorded_output_whose_subject_holds_a_mark_and_a_colon_is_read_the_same() {
    let awkward = case("awkward-subject");
    // Git writes the subject of the commit before on the line of a pair.
    let subject = "Fix: the lexer ! 2:  is not a pair";
    record(
        awkward,
        || rewritten((subject, "Add the lexer"), false),
        &range_diff_arguments(&request()),
    );
    check(awkward, |output| parse_pairs(output).unwrap());
    assert_eq!(
        kinds("awkward-subject"),
        [PairKind::Same, PairKind::Changed, PairKind::Same, PairKind::Dropped]
    );
    let output = std::fs::read(golden_path("awkward-subject")).unwrap();
    assert_eq!(parse_pairs(&output).unwrap()[1].subject, subject);
}

/// Two lines of 1,200 empty commits each, `long` and `other`, made at once by `git fast-import`.
fn long_lines() -> Repository {
    let repository = Repository::new();
    let mut stream = String::new();
    for (branch, mark) in [("long", 1), ("other", 2)] {
        for index in 0..1_200 {
            let _ = write!(
                stream,
                "commit refs/heads/{branch}\ncommitter Gitardo <gitardo@example.com> {index} +0000\ndata 2\n{mark}\n\n"
            );
        }
    }
    let mut child = repository
        .command("git")
        .args(["fast-import", "--quiet"])
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stream.as_bytes()).unwrap();
    assert!(child.wait().unwrap().success());
    repository
}

#[test]
fn range_diff_read_pairs__two_lines_of_1200_commits_are_not_compared_and_the_error_says_how_many() {
    let repository = long_lines();
    let request = RangeDiffRequest {
        ranges: Ranges {
            arguments: vec![String::from("long...other")],
            before: Line {
                from: String::from("other"),
                to: String::from("long"),
            },
            now: Line {
                from: String::from("long"),
                to: String::from("other"),
            },
        },
    };
    match read_pairs(&git_of(&repository), &request) {
        Err(GitError::Refused { reason }) => assert!(reason.contains("1,200 and 1,200"), "{reason}"),
        other => panic!("{other:?}"),
    }
}
