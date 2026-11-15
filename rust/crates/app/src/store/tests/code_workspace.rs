//! `repo.open-code-workspace`: the folders a workspace file of VS Code lists, opened as a workspace.

use std::path::{Path, PathBuf};

use views::Intent;
use views::page::PageState;

use super::{answer, folder, started, view_data, window};
use crate::store::feature::Said;
use crate::store::{JobKind, Message, State, code_workspace_folders, reduce};

/// The workspace file of these tests, `/work/all.code-workspace`.
fn file() -> PathBuf {
    folder("all.code-workspace")
}

/// Chooses a workspace file with this text in the system's dialog.
fn choose(state: &mut State, text: &str) -> Vec<crate::store::Effect> {
    reduce(
        state,
        Message::WorkspaceFileChosen(Some((file(), Ok(String::from(text))))),
    )
}

/// The folders whose repositories the effects look for.
fn looked_for(effects: &[crate::store::Effect]) -> Vec<PathBuf> {
    started(effects)
        .into_iter()
        .find_map(|(_, kind)| match kind {
            JobKind::FindRepositories { folders } => Some(folders),
            _ => None,
        })
        .unwrap_or_default()
}

#[test]
fn repo_open_code_workspace__api_and_dot_dot_web_open_work_api_and_web_in_that_order() {
    let mut state = window();
    let effects = choose(
        &mut state,
        r#"{ "folders": [{ "path": "api" }, { "path": "../web" }] }"#,
    );
    let (api, web) = (folder("api"), PathBuf::from("/web"));
    assert_eq!(looked_for(&effects), [api.clone(), web.clone()]);
    let found = [
        (api.as_path(), std::slice::from_ref(&api)),
        (Path::new("/web"), std::slice::from_ref(&web)),
    ];
    let _opened = answer(&mut state, effects, &found);
    let tabs: Vec<PathBuf> = state.tabs.open.iter().map(|tab| tab.folder.clone()).collect();
    assert_eq!(tabs, [api, web]);
}

#[test]
fn repo_open_code_workspace__comments_and_trailing_commas_are_no_error() {
    let text = r#"// The repositories of the team.
{
    "folders": [
        { "path": "api" }, // the server
        { "path": "../web", },
    ],
    /* Not read. */
    "settings": {},
}"#;
    assert_eq!(
        code_workspace_folders(text, &file()),
        Ok(vec![folder("api"), PathBuf::from("/web")])
    );
}

#[test]
fn repo_open_code_workspace__a_file_with_no_folders_is_refused() {
    let mut state = window();
    assert_eq!(choose(&mut state, r#"{ "settings": {} }"#), []);
    assert_eq!(
        state.operations.said,
        Some(Said::error(String::from("This file names no folders.")))
    );
    assert!(state.workspace.is_none());
}

#[test]
fn repo_open_code_workspace__the_workspace_opened_is_the_first_recent_one_named_after_the_file() {
    let mut state = window();
    let api = folder("api");
    let effects = choose(&mut state, r#"{ "folders": [{ "path": "api" }] }"#);
    let _opened = answer(&mut state, effects, &[(&api, std::slice::from_ref(&api))]);
    assert_eq!(state.persisted.recent[0].file, Some(file()));
    let _closed = reduce(&mut state, Message::Intent(Intent::CloseTab(0)));
    let data = view_data(&state);
    let views::Content::Page(host) = data.window_state(&state).content else {
        panic!("the Welcome page is shown");
    };
    let PageState::Welcome(welcome) = host.page else {
        panic!("the Welcome page is shown");
    };
    assert_eq!(welcome.recent.first().map(String::as_str), Some("all"));
}
