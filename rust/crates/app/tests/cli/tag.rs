use testkit::{Remote, Repository};

use crate::{cli, cli_done, revision};

fn repository() -> Repository {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a")]);
    repository
}

#[test]
fn tag_create__an_empty_message_makes_a_lightweight_tag() {
    let repository = repository();
    let head = revision(&repository, "HEAD");
    let target = format!("commit:{head}");
    let ended = cli_done(
        &repository,
        &["--target", &target, "tag.create", "name=v1.0.0", "message="],
    );
    assert!(ended.out.contains(&format!("> git tag v1.0.0 {head}")), "{}", ended.out);
    assert_eq!(repository.git(&["cat-file", "-t", "v1.0.0"]).trim(), "commit");
}

#[test]
fn tag_create__a_message_makes_an_annotated_tag_dated_when_it_was_made() {
    let repository = repository();
    let head = revision(&repository, "HEAD");
    let target = format!("commit:{head}");
    let ended = cli_done(
        &repository,
        &[
            "--target",
            &target,
            "tag.create",
            "name=v1.0.0",
            "message=First release",
        ],
    );
    assert!(
        ended
            .out
            .contains(&format!("> git tag -a v1.0.0 -m \"First release\" {head}")),
        "{}",
        ended.out
    );
    assert_eq!(repository.git(&["cat-file", "-t", "v1.0.0"]).trim(), "tag");
    let dated = repository.git(&["for-each-ref", "--format=%(taggerdate:unix)", "refs/tags/v1.0.0"]);
    assert!(dated.trim().parse::<i64>().is_ok_and(|date| date > 0), "{dated}");
}

#[test]
fn tag_create__backing_out_of_the_message_runs_nothing() {
    let repository = repository();
    let head = revision(&repository, "HEAD");
    let target = format!("commit:{head}");
    let ended = cli(&repository, &["--target", &target, "tag.create", "name=v1.0.0"]);
    assert_eq!(ended.code, Some(2));
    assert_eq!(repository.git(&["tag", "--list"]), "");
}

#[test]
fn tag_delete__asks_then_deletes_and_the_remote_keeps_it() {
    let repository = repository();
    let remote = Remote::new(&repository);
    repository.tag("v1.0.0");
    let _pushed = repository.git(&["push", "--quiet", "origin", "refs/tags/v1.0.0"]);
    let asked = cli(&repository, &["--target", "tag:v1.0.0", "tag.delete"]);
    assert_eq!(asked.code, Some(2), "it warns first: {}", asked.out);
    let ended = cli_done(&repository, &["--yes", "--target", "tag:v1.0.0", "tag.delete"]);
    assert!(ended.out.contains("> git tag -d v1.0.0"), "{}", ended.out);
    assert!(!ended.out.contains("git push"), "{}", ended.out);
    assert_eq!(repository.git(&["tag", "--list"]), "");
    assert_eq!(remote.git(&["tag", "--list"]), "v1.0.0\n");
}

#[test]
fn tag_push__pushes_the_tag_to_origin() {
    let repository = repository();
    let remote = Remote::new(&repository);
    let _upstream = repository.git(&["remote", "add", "upstream", &remote.path().to_string_lossy()]);
    repository.tag("v1.0.0");
    let ended = cli_done(&repository, &["--target", "tag:v1.0.0", "tag.push"]);
    assert!(
        ended.out.contains("> git push origin refs/tags/v1.0.0"),
        "{}",
        ended.out
    );
    assert_eq!(remote.git(&["tag", "--list"]), "v1.0.0\n");
}
