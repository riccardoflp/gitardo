//! The scripts of the `package.json` files of a worktree (`tasks.read`): one group for each file, each script run
//! with the package manager the lock file beside the file, or the nearest above it, names.

use json::Value;
use model::tasks::{
    MODULES_FOLDER, PACKAGE_FILE, PackageFile, PackageScripts, Task, TaskFileProblem, TaskKind, TaskReference,
    TasksContext, Word, package_manager,
};

use super::read::why;

/// The scripts of every `package.json` of the files given, the one at the top first, then by folder; and the files
/// that are not JSON, which list nothing. A file under `node_modules` is one of a package installed, and a file with
/// no script has no group.
pub(super) fn package_scripts(context: &TasksContext) -> (Vec<PackageScripts>, Vec<TaskFileProblem>) {
    let mut files: Vec<(String, &PackageFile)> = context
        .files
        .packages
        .iter()
        .filter_map(|file| Some((folder_of(&file.path)?, file)))
        .collect();
    files.sort_by(|(first, _), (second, _)| first.cmp(second));
    files.dedup_by(|(first, _), (second, _)| first == second);
    let mut packages = Vec::new();
    let mut problems = Vec::new();
    for (folder, file) in files {
        match scripts_of(&folder, file, context) {
            Ok(Some(package)) => packages.push(package),
            Ok(None) => {}
            Err(error) => problems.push(TaskFileProblem {
                file: file.path.replace('\\', "/"),
                why: why(&error),
            }),
        }
    }
    (packages, problems)
}

/// The folder of a `package.json` from the top of the worktree, with `/`, empty for the one at the top; none for a
/// file that is not a `package.json`, or that is under `node_modules`.
fn folder_of(path: &str) -> Option<String> {
    let path = path.replace('\\', "/");
    let path = path.trim_start_matches("./");
    let (folder, name) = path.rsplit_once('/').unwrap_or(("", path));
    let is_installed = folder.split('/').any(|part| part == MODULES_FOLDER);
    (name == PACKAGE_FILE && !is_installed).then(|| folder.trim_matches('/').to_owned())
}

/// The scripts of one file; none when it has none.
fn scripts_of(folder: &str, file: &PackageFile, context: &TasksContext) -> Result<Option<PackageScripts>, json::Error> {
    let root = json::parse(&file.text)?;
    let members = root.get("scripts").and_then(Value::as_object).unwrap_or_default();
    let manager = package_manager(folder, &context.files.lock_files);
    let directory = folder
        .split('/')
        .filter(|part| !part.is_empty())
        .fold(context.folder.clone(), |directory, part| directory.join(part));
    let scripts: Vec<Task> = members
        .iter()
        .filter_map(|member| {
            let text = member.value.as_str()?;
            let mut task = Task::named(TaskReference::npm(folder, &member.name), directory.clone());
            task.kind = TaskKind::Shell;
            task.command = Some(Word::plain(manager.program()));
            task.arguments = vec![Word::plain("run"), Word::plain(&member.name)];
            task.script = Some(text.to_owned());
            Some(task)
        })
        .collect();
    if scripts.is_empty() {
        return Ok(None);
    }
    let name = if folder.is_empty() {
        let name = context.folder.file_name().unwrap_or_default();
        name.to_string_lossy().into_owned()
    } else {
        folder.to_owned()
    };
    Ok(Some(PackageScripts {
        folder: folder.to_owned(),
        name,
        manager,
        scripts,
    }))
}
