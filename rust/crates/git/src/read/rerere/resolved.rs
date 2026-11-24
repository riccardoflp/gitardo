//! The files git resolved as before while an operation is stopped (`rerere.read-resolved`), and a file as git marked it
//! when it stopped (`rerere.read-marked`).

use std::ffi::OsString;
use std::path::Path;

use model::rerere::{MarkedRequest, ResolvedAsBefore, ResolvedRequest};

use crate::read::fields::arguments;
use crate::{Git, GitError};

/// The arguments that list the files in conflict git did not resolve: `git rerere remaining`.
#[must_use]
pub fn remaining_arguments() -> Vec<OsString> {
    arguments(&["rerere", "remaining"])
}

/// The paths `git rerere remaining` printed, one to a line, as git prints them, with no `-z`.
#[must_use]
pub fn parse_remaining(printed: &str) -> Vec<String> {
    printed
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The arguments that list the files whose conflicts were resolved and staged during the operation under way, which
/// the index keeps until the next commit: `git ls-files --resolve-undo -z`.
#[must_use]
pub fn resolve_undo_arguments() -> Vec<OsString> {
    arguments(&["ls-files", "--resolve-undo", "-z"])
}

/// The paths `git ls-files --resolve-undo -z` printed, `<mode> <id> <stage>\t<path>` for each version, each once.
#[must_use]
pub fn parse_resolve_undo(printed: &[u8]) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    for entry in printed.split(|byte| *byte == 0) {
        let entry = String::from_utf8_lossy(entry);
        if let Some((_, path)) = entry.split_once('\t')
            && !paths.iter().any(|known| known == path)
        {
            paths.push(path.to_owned());
        }
    }
    paths
}

/// Whether the content of a file is a resolution git recorded: a `postimage`, or `postimage.<variant>`, of the folder
/// `rr-cache`.
fn is_recorded(rr_cache: &Path, content: &[u8]) -> bool {
    let Ok(conflicts) = std::fs::read_dir(rr_cache) else {
        return false;
    };
    conflicts.flatten().any(|conflict| {
        std::fs::read_dir(conflict.path()).is_ok_and(|images| {
            images.flatten().any(|image| {
                image.file_name().to_string_lossy().starts_with("postimage")
                    && std::fs::read(image.path()).is_ok_and(|recorded| recorded == content)
            })
        })
    })
}

/// The files git resolved as before: those in conflict that `git rerere remaining` does not list, and those staged
/// during the operation, as `rerere.autoUpdate` has git do, each only when its content is a resolution git recorded,
/// so that a resolution forgotten is not said to be one whatever git lists. A path with a
/// line ending in it cannot be told apart in what git prints, and is taken as not resolved (Q1200). Git lists only the
/// conflicts it took note of: when rerere was off as the operation stopped, and turned on after, it wrote no `MERGE_RR`
/// and lists none, though it resolved none, so without that file no file in conflict is resolved as before.
///
/// # Errors
///
/// When git cannot be started, or `git rerere remaining` or `git ls-files` fails.
///
/// Feature: rerere.read-resolved
pub fn read_resolved(git: &Git, request: &ResolvedRequest) -> Result<Vec<ResolvedAsBefore>, GitError> {
    let mut resolved = Vec::new();
    let rr_cache = super::rr_cache(&request.git_folder);
    let holds_recorded = |path: &str| {
        let content = std::fs::read(git.directory().join(path)).unwrap_or_default();
        is_recorded(&rr_cache, &content)
    };
    if !request.conflicted.is_empty() && request.git_folder.join("MERGE_RR").is_file() {
        let printed = git.run_read(&remaining_arguments())?;
        let remaining = parse_remaining(&String::from_utf8_lossy(&printed));
        resolved.extend(
            request
                .conflicted
                .iter()
                .filter(|path| !path.contains('\n') && !remaining.contains(path) && holds_recorded(path))
                .map(|path| ResolvedAsBefore {
                    path: path.clone(),
                    is_staged: false,
                }),
        );
    }
    let staged = parse_resolve_undo(&git.run_read(&resolve_undo_arguments())?);
    for path in staged {
        if request.conflicted.contains(&path) {
            continue;
        }
        if holds_recorded(&path) {
            resolved.push(ResolvedAsBefore { path, is_staged: true });
        }
    }
    Ok(resolved)
}

/// The arguments that make again a file as git marked it:
/// `git merge-file -p --object-id -L <current> -L base -L <incoming> :2:<path> <base> :3:<path>`.
#[must_use]
pub fn marked_arguments(request: &MarkedRequest, base: &str) -> Vec<OsString> {
    let mut listed = arguments(&["merge-file", "-p", "--object-id", "-L"]);
    listed.push(OsString::from(&request.current));
    listed.extend(arguments(&["-L", "base", "-L"]));
    listed.push(OsString::from(&request.incoming));
    listed.push(OsString::from(format!(":2:{}", request.path)));
    listed.push(OsString::from(base));
    listed.push(OsString::from(format!(":3:{}", request.path)));
    listed
}

/// The text of a file in conflict as git marked it when it stopped, made again from the versions of the index: its
/// base, `:1:<path>`, or for a file both sides added the id of an empty file, which `git hash-object --stdin` gives
/// without writing it. Git ends with the number of conflicts, which is not a failure. Nothing on disk changes.
///
/// # Errors
///
/// When git cannot be started, or a version is not in the index, as for a file git staged.
///
/// Feature: rerere.read-marked
pub fn read_marked(git: &Git, request: &MarkedRequest) -> Result<Vec<u8>, GitError> {
    let literal = format!(":(literal){}", request.path);
    let stages = git.run_read(&arguments(&["ls-files", "-u", "-z", "--", &literal]))?;
    let has_base = stages
        .split(|byte| *byte == 0)
        .any(|entry| String::from_utf8_lossy(entry).contains(" 1\t"));
    let base = if has_base {
        format!(":1:{}", request.path)
    } else {
        let empty = git.read_output_with_input(&arguments(&["hash-object", "--stdin"]), &[])?;
        String::from_utf8_lossy(&empty.stdout).trim().to_owned()
    };
    let listed = marked_arguments(request, &base);
    let output = git.read_output(&listed)?;
    // `git merge-file` ends with the number of conflicts, at most 127; git ends with 128 or more when it fails.
    match output.code {
        Some(code) if (0..=127).contains(&code) => Ok(output.stdout),
        code => Err(GitError::Failed {
            command: crate::command_line(&listed),
            code,
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        }),
    }
}
