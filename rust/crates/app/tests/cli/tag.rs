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

/// A repository whose `origin` has the tag `v1`, made here.
fn with_v1_pushed() -> (Repository, Remote) {
    let repository = repository();
    let remote = Remote::new(&repository);
    repository.tag("v1");
    let _pushed = repository.git(&["push", "--quiet", "origin", "refs/tags/v1"]);
    (repository, remote)
}

#[test]
fn tag_delete_remote__delete_from_origin_asks_then_the_remote_loses_v1_and_this_repository_keeps_it() {
    let (repository, remote) = with_v1_pushed();
    let asked = cli(&repository, &["--target", "tag:v1", "tag.delete-remote"]);
    assert_eq!(asked.code, Some(2), "it warns first: {}", asked.out);
    let ended = cli_done(&repository, &["--yes", "--target", "tag:v1", "tag.delete-remote"]);
    assert!(
        ended.out.contains("> git push origin --delete refs/tags/v1"),
        "{}",
        ended.out
    );
    assert_eq!(remote.git(&["tag", "--list"]), "");
    assert_eq!(repository.git(&["tag", "--list"]), "v1\n");
}

#[test]
fn tag_delete_remote__a_branch_of_the_name_of_the_tag_on_origin_is_left_there() {
    let (repository, remote) = with_v1_pushed();
    let _pushed = repository.git(&["push", "--quiet", "origin", "main:refs/heads/v1"]);
    let _ended = cli_done(&repository, &["--yes", "--target", "tag:v1", "tag.delete-remote"]);
    assert_eq!(remote.git(&["branch", "--list", "v1"]).trim(), "v1");
}

#[test]
fn tag_push_all__with_v1_on_origin_the_tags_made_here_are_pushed_with_tags_and_origin_has_the_three() {
    let (repository, remote) = with_v1_pushed();
    repository.tag("v2");
    repository.tag("v3");
    let ended = cli_done(&repository, &["tag.push-all", "tags=v2,v3"]);
    assert!(ended.out.contains("> git push origin --tags"), "{}", ended.out);
    assert_eq!(remote.git(&["tag", "--list"]), "v1\nv2\nv3\n");
}

#[test]
fn tag_push_all__v3_unticked_pushes_v2_named_in_full() {
    let (repository, remote) = with_v1_pushed();
    repository.tag("v2");
    repository.tag("v3");
    let ended = cli_done(&repository, &["tag.push-all", "tags=v2"]);
    assert!(ended.out.contains("> git push origin refs/tags/v2"), "{}", ended.out);
    assert_eq!(remote.git(&["tag", "--list"]), "v1\nv2\n");
}

#[test]
fn tag_push_all__v1_moved_here_is_not_pushed_and_the_others_are_named_in_full() {
    let (mut repository, remote) = with_v1_pushed();
    let _second = repository.commit("Change a", &[("a.txt", "a, again")]);
    let _moved = repository.git(&["tag", "--force", "v1"]);
    repository.tag("v2");
    let ended = cli_done(&repository, &["tag.push-all", "tags=v2"]);
    assert!(ended.out.contains("> git push origin refs/tags/v2"), "{}", ended.out);
    let on_origin = remote.git(&["rev-parse", "refs/tags/v1"]);
    assert_ne!(on_origin, repository.git(&["rev-parse", "refs/tags/v1"]));
}

#[test]
fn tag_push_all__with_every_tag_on_origin_it_says_so_and_pushes_nothing() {
    let (repository, _remote) = with_v1_pushed();
    let ended = cli(&repository, &["tag.push-all"]);
    let said = format!("{}{}", ended.out, ended.errors);
    assert!(said.contains("origin has every tag of this repository."), "{said}");
    assert!(!said.contains("> git push"), "{said}");
}
