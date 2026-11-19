//! The reads of the hosting service of the main remote that version 1.11 adds for milestone M37: the service and the
//! project, the address of a page, and the merge requests of a GitLab, whose token is asked of the extension by its host
//! name. Nothing is asked of any service: the extension gives no token.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use protocol::messages::VERSION;
use serde_json::{Value, json};
use testkit::Repository;

use crate::harness::{Engine, folder, repository};
use crate::reads::result;

/// An engine that answers `host/githubToken`, with the settings given, and a repository opened.
fn opened_with_settings(repository: &Repository, settings: &Value) -> Engine {
    let mut engine = Engine::start();
    let initialized = engine.ask(
        "initialize",
        &json!({
            "protocol": VERSION,
            "client": { "name": "gitardo-vscode", "version": "0.0.0" },
            "settings": settings,
            "host": { "githubToken": true },
            "logLevel": "info"
        }),
    );
    assert!(initialized.get("result").is_some(), "{initialized}");
    let _ = engine.ask("repository/open", &json!({ "repository": folder(repository) }));
    engine
}

#[test]
fn hosting_detect_remote__the_engine_says_the_service_of_the_main_remote_and_a_gitlab_of_ones_own() {
    let repository = repository();
    let _ = repository.git(&["remote", "add", "origin", "git@git.example.test:group/app.git"]);
    let settings = json!({ "hosting.selfHosted": { "git.example.test": { "kind": "gitlab", "web": "https://code.example.test" } } });
    let mut engine = opened_with_settings(&repository, &settings);
    let found = result(&mut engine, &repository, "read/hosting.detect-remote", json!({}));
    assert_eq!(found["kind"], json!("found"), "{found}");
    assert_eq!(found["service"], json!("gitLab"));
    assert_eq!(found["web"], json!("https://code.example.test/group/app"));
    assert_eq!(found["canCheckOut"], json!(true));
    let link = result(
        &mut engine,
        &repository,
        "read/hosting.web-link",
        json!({ "page": { "kind": "create", "branch": "feature" } }),
    );
    let url = link["url"].as_str().unwrap_or_default();
    assert!(
        url.starts_with("https://code.example.test/group/app/-/merge_requests/new"),
        "{url}"
    );
    let _ = repository.git(&["remote", "set-url", "origin", "https://bitbucket.org/owner/app.git"]);
    let bitbucket = result(&mut engine, &repository, "read/hosting.detect-remote", json!({}));
    assert_eq!(bitbucket["service"], json!("bitbucket"));
    assert_eq!(bitbucket["comparesSinceDiverged"], json!(true));
    let _ = repository.git(&["remote", "set-url", "origin", "https://example.test/owner/app.git"]);
    let none = result(&mut engine, &repository, "read/hosting.detect-remote", json!({}));
    assert_eq!(none, json!({ "kind": "none" }));
}

#[test]
fn hosting_read_merge_requests__the_extension_is_asked_for_the_token_of_the_gitlab_by_its_host_name() {
    let repository = repository();
    let _ = repository.git(&["remote", "add", "origin", "https://gitlab.com/group/app.git"]);
    let mut engine = opened_with_settings(&repository, &json!({}));
    let params = json!({ "remote": null, "repository": folder(&repository) });
    let id = engine.send("read/github.read-pull-requests", &params);
    let mut asked = Vec::new();
    let answer = engine.answer_asked(id, |request| {
        asked.push(request.clone());
        json!({ "token": null })
    });
    assert_eq!(answer["result"], json!({ "kind": "noToken" }), "{answer}");
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0]["method"], json!("host/githubToken"));
    assert_eq!(asked[0]["params"], json!({ "host": "gitlab.com", "ask": false }));
}
