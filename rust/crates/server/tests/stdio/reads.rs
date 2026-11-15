//! The reads of stage 5 that look at commits and files, each asked as the extension asks it and answered from a
//! throwaway repository.

use protocol::reads::BytesJson;
use serde_json::{Value, json};
use testkit::Repository;

use crate::harness::{Engine, folder, repository};

/// An engine started, initialized, with a repository opened.
pub fn opened(repository: &Repository) -> Engine {
    let mut engine = Engine::start();
    let _ = engine.initialize();
    let opened = engine.ask("repository/open", &json!({ "repository": folder(repository) }));
    assert!(opened.get("result").is_some(), "{opened}");
    engine
}

/// The answer of a read of the repository, with the parameters given besides `repository`.
pub fn read(engine: &mut Engine, repository: &Repository, method: &str, params: Value) -> Value {
    let mut params = params;
    if params.is_null() {
        params = json!({});
    }
    params["repository"] = json!(folder(repository));
    engine.ask(method, &params)
}

/// The result of a read that must succeed.
pub fn result(engine: &mut Engine, repository: &Repository, method: &str, params: Value) -> Value {
    let answer = read(engine, repository, method, params);
    assert!(answer.get("error").is_none(), "{method}: {answer}");
    answer["result"].clone()
}

/// The bytes of a `Bytes` of an answer.
pub fn bytes(value: &Value) -> Vec<u8> {
    serde_json::from_value::<BytesJson>(value.clone())
        .unwrap()
        .bytes()
        .unwrap()
}

/// The full id of a revision.
fn id(repository: &Repository, revision: &str) -> String {
    repository.git(&["rev-parse", revision]).trim_end().to_owned()
}

#[test]
fn a_revision_is_its_full_id_or_null_for_the_search_and_for_a_comparison() {
    let repository = repository();
    let mut engine = opened(&repository);
    for method in ["read/search.read-revision", "read/compare.resolve-revision"] {
        let found = result(&mut engine, &repository, method, json!({ "revision": "feature" }));
        assert_eq!(found["commit"], json!(id(&repository, "feature")), "{method}");
        let none = result(
            &mut engine,
            &repository,
            method,
            json!({ "revision": "no-such-branch" }),
        );
        assert_eq!(none["commit"], Value::Null, "{method}");
    }
}

#[test]
fn the_details_of_a_commit_of_a_missing_stash_and_of_the_uncommitted_changes() {
    let repository = repository();
    std::fs::write(repository.path().join("new.txt"), "new\n").unwrap();
    let mut engine = opened(&repository);
    let head = id(&repository, "HEAD");
    let detail = result(
        &mut engine,
        &repository,
        "read/details.read-commit",
        json!({ "commit": head }),
    );
    assert_eq!(detail["id"], json!(head));
    assert_eq!(detail["message"].as_str().map(str::trim_end), Some("Change a"));
    assert_eq!(detail["files"][0]["path"], json!("a.txt"));
    assert_eq!(detail["parents"], json!([id(&repository, "HEAD~1")]));
    let stash = result(
        &mut engine,
        &repository,
        "read/details.read-commit",
        json!({ "stash": 0 }),
    );
    assert_eq!(stash, Value::Null);
    let uncommitted = result(
        &mut engine,
        &repository,
        "read/details.read-commit",
        json!({ "uncommitted": true }),
    );
    assert_eq!(uncommitted["id"], Value::Null);
    assert_eq!(
        uncommitted["files"],
        json!([{ "path": "new.txt", "originalPath": null, "kind": "added" }]),
        "{uncommitted}"
    );
}

#[test]
fn a_commit_named_by_anything_but_its_full_id_answers_32602() {
    let repository = repository();
    let mut engine = opened(&repository);
    for (method, params) in [
        ("read/details.read-commit", json!({ "commit": "main" })),
        ("read/commit-page.read-patch", json!({ "commit": "HEAD", "path": null })),
    ] {
        let answer = read(&mut engine, &repository, method, params);
        assert_eq!(answer["error"]["code"], json!(-32_602), "{method}: {answer}");
    }
}

#[test]
fn the_patch_of_a_commit_has_the_lines_it_changed_in_each_file() {
    let repository = repository();
    let mut engine = opened(&repository);
    let patch = result(
        &mut engine,
        &repository,
        "read/commit-page.read-patch",
        json!({ "commit": id(&repository, "HEAD"), "path": null }),
    );
    assert_eq!(patch["kind"], json!("files"), "{patch}");
    let file = &patch["files"][0];
    assert_eq!(file["path"], json!("a.txt"));
    assert_eq!((file["added"].clone(), file["removed"].clone()), (json!(1), json!(1)));
    let texts: Vec<&str> = file["hunks"][0]["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line["text"].as_str().unwrap())
        .collect();
    assert_eq!(texts, ["a", "a, again"]);
}

#[test]
fn a_comparison_has_its_sides_its_files_and_its_patch() {
    let repository = repository();
    let mut engine = opened(&repository);
    let (feature, main) = (id(&repository, "feature"), id(&repository, "main"));
    let sides = json!({
        "first": { "kind": "commit", "commit": feature },
        "second": { "kind": "commit", "commit": main },
        "mode": "sinceDiverged",
        "path": null
    });
    let view = result(&mut engine, &repository, "read/compare.read-sides", sides.clone());
    assert_eq!(view["mergeBase"], json!(feature), "{view}");
    assert_eq!(
        (view["onlyInFirst"].clone(), view["onlyInSecond"].clone()),
        (json!(0), json!(1))
    );
    assert_eq!(view["files"][0]["path"], json!("a.txt"));
    let files = result(
        &mut engine,
        &repository,
        "read/compare.read-files",
        json!({ "from": feature, "to": main }),
    );
    assert_eq!(files[0]["kind"], json!("modified"), "{files}");
    let mut patch = sides;
    patch["mergeBase"] = json!(feature);
    patch["files"] = files;
    patch["form"] = json!("text");
    let text = result(&mut engine, &repository, "read/compare.read-patch", patch);
    assert_eq!(text["kind"], json!("text"), "{text}");
    assert!(String::from_utf8(bytes(&text["text"])).unwrap().contains("+a, again"));
}

#[test]
fn the_log_of_what_is_checked_out_says_there_is_more_and_nothing_of_what_is_pushed_without_a_remote() {
    let repository = repository();
    let mut engine = opened(&repository);
    let log = result(&mut engine, &repository, "read/branch.read-log", json!({ "count": 1 }));
    assert_eq!(log["commits"].as_array().map(Vec::len), Some(1), "{log}");
    assert_eq!(log["commits"][0]["subject"], json!("Change a"));
    assert_eq!(log["hasMore"], json!(true));
    assert_eq!(
        (log["unpushed"].clone(), log["upstream"].clone()),
        (Value::Null, Value::Null)
    );
}

#[test]
fn the_working_tree_and_the_paths_git_ignores() {
    let mut repository = repository();
    let _ = repository.commit("Ignore the logs", &[(".gitignore", "*.log\n")]);
    std::fs::write(repository.path().join("a.txt"), "edited\n").unwrap();
    std::fs::write(repository.path().join("run.log"), "log\n").unwrap();
    let mut engine = opened(&repository);
    let tree = result(&mut engine, &repository, "read/changes.read-working-tree", Value::Null);
    assert_eq!(tree["unstaged"][0]["path"], json!("a.txt"), "{tree}");
    assert_eq!(tree["untracked"], json!([]));
    let ignored = result(
        &mut engine,
        &repository,
        "read/changes.read-ignored",
        json!({ "paths": ["a.txt", "run.log"] }),
    );
    assert_eq!(ignored["ignored"], json!(["run.log"]));
}

#[test]
fn the_sides_of_a_diff_and_a_file_as_a_commit_has_it() {
    let repository = repository();
    let mut engine = opened(&repository);
    let sides = result(
        &mut engine,
        &repository,
        "read/diff.read-sides",
        json!({
            "left": { "version": { "kind": "commit", "commit": id(&repository, "feature") }, "path": "a.txt" },
            "right": { "version": { "kind": "workingTree", "unsaved": BytesJson::of(b"typed\n") }, "path": "a.txt" }
        }),
    );
    assert_eq!(bytes(&sides["left"]), b"a\n");
    assert_eq!(bytes(&sides["right"]), b"typed\n");
    let file = result(
        &mut engine,
        &repository,
        "read/file-view.read-revision",
        json!({ "path": "a.txt", "revision": "feature" }),
    );
    assert_eq!(bytes(&file), b"a\n");
    let none = result(
        &mut engine,
        &repository,
        "read/file-view.read-revision",
        json!({ "path": "a.txt", "revision": "nothing" }),
    );
    assert_eq!(none, Value::Null);
}

#[test]
fn the_blame_of_a_line_and_of_a_file() {
    let repository = repository();
    let mut engine = opened(&repository);
    let line = result(
        &mut engine,
        &repository,
        "read/blame.read-line",
        json!({ "path": "a.txt", "line": 1, "contents": null }),
    );
    assert_eq!(line["kind"], json!("committed"), "{line}");
    assert_eq!(line["commit"], json!(id(&repository, "HEAD")));
    assert_eq!(line["summary"], json!("Change a"));
    let typed = BytesJson::of(b"a, again\ntyped\n");
    let file = result(
        &mut engine,
        &repository,
        "read/blame.read-file",
        json!({ "path": "a.txt", "revision": null, "contents": typed }),
    );
    let kinds: Vec<&Value> = file.as_array().unwrap().iter().map(|line| &line["kind"]).collect();
    assert_eq!(kinds, [&json!("committed"), &json!("notCommitted")], "{file}");
}

#[test]
fn the_names_of_a_file_and_where_its_lines_are_committed() {
    let mut repository = repository();
    let _ = repository.git(&["mv", "a.txt", "b.txt"]);
    let _ = repository.commit("Rename a", &[]);
    let mut engine = opened(&repository);
    let names = result(
        &mut engine,
        &repository,
        "read/file-history.read-names",
        json!({ "path": "b.txt" }),
    );
    assert_eq!(names["names"], json!(["b.txt", "a.txt"]), "{names}");
    let lines = result(
        &mut engine,
        &repository,
        "read/file-history.read-committed-lines",
        json!({ "path": "b.txt", "renamedFrom": null, "text": "a, again\n", "lines": { "start": 1, "end": 1 } }),
    );
    assert_eq!(lines["kind"], json!("committed"), "{lines}");
    assert_eq!(lines["lines"], json!({ "start": 1, "end": 1 }));
}

#[test]
fn the_common_folder_and_the_message_git_prepared() {
    let repository = repository();
    let mut engine = opened(&repository);
    let common = result(&mut engine, &repository, "read/repo.common-folder", Value::Null);
    let folder = std::path::PathBuf::from(common["folder"].as_str().unwrap());
    assert_eq!(
        folder.canonicalize().unwrap(),
        repository.path().join(".git").canonicalize().unwrap()
    );
    let none = result(&mut engine, &repository, "read/commit.prepared-message", Value::Null);
    assert_eq!(none["message"], Value::Null);
    std::fs::write(
        repository.path().join(".git").join("SQUASH_MSG"),
        "Squashed commit of feature\n\n",
    )
    .unwrap();
    let message = result(&mut engine, &repository, "read/commit.prepared-message", Value::Null);
    assert_eq!(message["message"], json!("Squashed commit of feature"));
}
