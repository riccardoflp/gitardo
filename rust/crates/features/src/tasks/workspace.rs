//! The tasks of `.vscode/tasks.json` (`tasks.read`): each with what it runs on the system at hand, its options, the
//! tasks it depends on and what says it is ready, read as VS Code reads version 2.0.0 of the file.

use std::path::PathBuf;

use json::{Key, Value};
use model::tasks::{
    Background, DependsOrder, PackageScripts, Quoting, Task, TaskCategory, TaskKind, TaskReference, TaskShell,
    TasksContext, Word,
};

use super::variables::resolved;

/// The tasks of the text of a `tasks.json`, in its order, those with `hide` included. A text with nothing in it has
/// no tasks.
///
/// # Errors
///
/// When the text is not JSON with comments.
pub(super) fn workspace_tasks(
    text: &str,
    context: &TasksContext,
    packages: &[PackageScripts],
) -> Result<Vec<Task>, json::Error> {
    if text.trim_start_matches('\u{feff}').trim().is_empty() {
        return Ok(Vec::new());
    }
    let root = json::parse_with_comments(text)?;
    let entries = root.get("tasks").and_then(Value::as_array).unwrap_or_default();
    let names: Vec<Option<String>> = entries.iter().map(name_of).collect();
    let file = File {
        text,
        context,
        packages,
        root: &root,
        names: &names,
    };
    Ok(entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| file.task(index, entry))
        .collect())
}

/// A `tasks.json` being read.
struct File<'a> {
    /// Its text, for the line of each task.
    text: &'a str,
    /// What its variables stand for.
    context: &'a TasksContext,
    /// The scripts of the worktree, which a task may depend on.
    packages: &'a [PackageScripts],
    /// The whole file, whose `options` hold for every task.
    root: &'a Value,
    /// The names of its tasks, none for one that cannot be named.
    names: &'a [Option<String>],
}

impl File<'_> {
    /// The task of an entry of `tasks`; none for an entry with nothing to name it by.
    fn task(&self, index: usize, entry: &Value) -> Option<Task> {
        let name = self.names.get(index)?.clone()?;
        let system = self.context.system.override_name();
        let layers = [
            Some(Layer::options_of(self.root)),
            self.root.get(system).map(Layer::options_of),
            Some(Layer::of(entry)),
            entry.get(system).map(Layer::of),
        ];
        let layer = layers.into_iter().flatten().fold(Layer::default(), Layer::under);
        let mut task = Task::named(TaskReference::workspace(&name), self.context.folder.clone());
        task.kind = kind_of(entry);
        self.fill_command(&mut task, layer);
        task.category = category_of(entry);
        task.depends_on = self.depends_on(entry);
        if entry.get("dependsOrder").and_then(Value::as_str) == Some("sequence") {
            task.depends_order = DependsOrder::Sequence;
        }
        task.is_hidden = entry.get("hide").and_then(Value::as_bool) == Some(true);
        if entry.get("isBackground").and_then(Value::as_bool) == Some(true) {
            task.background = Some(background_of(entry));
        }
        task.line = self.line(index);
        Some(task)
    }

    /// Fills what a task runs from what the file says of it, its variables resolved.
    fn fill_command(&self, task: &mut Task, layer: Layer) {
        let mut unresolved = Vec::new();
        let mut resolve = |text: &str| resolved(text, self.context, &mut unresolved);
        let mut word = |word: Word| Word {
            value: resolve(&word.value),
            quoting: word.quoting,
        };
        task.command = layer.command.map(&mut word);
        task.arguments = layer.arguments.unwrap_or_default().into_iter().map(&mut word).collect();
        if let Some(folder) = layer.folder {
            // A folder that does not start from the root of the disk starts from the worktree shown.
            task.folder = self.context.folder.join(PathBuf::from(resolve(&folder)));
        }
        task.environment = layer
            .environment
            .into_iter()
            .map(|(name, value)| (name, resolve(&value)))
            .collect();
        task.shell = layer.shell_executable.map(|executable| TaskShell {
            executable: resolve(&executable),
            arguments: layer
                .shell_arguments
                .map(|arguments| arguments.iter().map(|argument| resolve(argument)).collect()),
        });
        task.unresolved = unresolved;
    }

    /// The tasks an entry depends on: each a label, or what identifies a task of another kind.
    fn depends_on(&self, entry: &Value) -> Vec<TaskReference> {
        let named = match entry.get("dependsOn") {
            Some(Value::Array(items)) => items.iter().collect(),
            Some(one) => vec![one],
            None => Vec::new(),
        };
        named.into_iter().filter_map(|named| self.reference(named)).collect()
    }

    /// The task a `dependsOn` names. A label is a task of the file when it has one of that label, else the script
    /// VS Code names so (`npm: build`, `npm: build - extension`), else a task of the file that is missing.
    fn reference(&self, named: &Value) -> Option<TaskReference> {
        let label = match named {
            Value::String(label) => label.clone(),
            Value::Object(_) => return identified(named),
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::Array(_) => return None,
        };
        let is_of_file = self.names.iter().flatten().any(|name| *name == label);
        let script = label.strip_prefix("npm: ").filter(|_| !is_of_file).and_then(|script| {
            self.scripts()
                .find(|reference| reference.name_in_vs_code() == script)
                .cloned()
        });
        Some(script.unwrap_or_else(|| TaskReference::workspace(&label)))
    }

    /// The scripts of the worktree, by how they are named.
    fn scripts(&self) -> impl Iterator<Item = &TaskReference> {
        self.packages
            .iter()
            .flat_map(|package| &package.scripts)
            .map(|script| &script.reference)
    }

    /// The line of the label of a task, or of the task when it has no label.
    fn line(&self, index: usize) -> Option<usize> {
        let entry = [Key::Member(String::from("tasks")), Key::Index(index)];
        let label = [entry.as_slice(), &[Key::Member(String::from("label"))]].concat();
        json::position_of(self.text, &label)
            .or_else(|| json::position_of(self.text, &entry))
            .map(|position| position.line)
    }
}

/// What a level of `tasks.json` says a task runs: the file as a whole, a task, or either for one system.
#[derive(Default)]
struct Layer {
    /// `command`.
    command: Option<Word>,
    /// `args`.
    arguments: Option<Vec<Word>>,
    /// `options.cwd`.
    folder: Option<String>,
    /// `options.env`, in the order written.
    environment: Vec<(String, String)>,
    /// `options.shell.executable`.
    shell_executable: Option<String>,
    /// `options.shell.args`.
    shell_arguments: Option<Vec<String>>,
}

impl Layer {
    /// The command, the arguments and the options of a task, or of a task for one system.
    fn of(value: &Value) -> Layer {
        let arguments = value.get("args").and_then(Value::as_array);
        Layer {
            command: value.get("command").and_then(word_of),
            arguments: arguments.map(|items| items.iter().filter_map(word_of).collect()),
            ..Layer::options_of(value)
        }
    }

    /// The options alone, which is all the file as a whole gives its tasks.
    fn options_of(value: &Value) -> Layer {
        let options = value.get("options");
        let member = |name: &str| options.and_then(|options| options.get(name));
        let strings = |items: &[Value]| items.iter().filter_map(Value::as_str).map(str::to_owned).collect();
        let environment = member("env").and_then(Value::as_object).unwrap_or_default();
        let shell = member("shell");
        Layer {
            command: None,
            arguments: None,
            folder: member("cwd").and_then(Value::as_str).map(str::to_owned),
            environment: environment
                .iter()
                .filter_map(|member| Some((member.name.clone(), member.value.as_str()?.to_owned())))
                .collect(),
            shell_executable: shell
                .and_then(|shell| shell.get("executable"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            shell_arguments: shell
                .and_then(|shell| shell.get("args"))
                .and_then(Value::as_array)
                .map(strings),
        }
    }

    /// This level over a wider one: what it says replaces what the wider one says, and its environment is added to
    /// the wider one's, a name of both taking its value.
    fn under(wider: Layer, layer: Layer) -> Layer {
        let mut environment = wider.environment;
        for (name, value) in layer.environment {
            match environment.iter_mut().find(|(known, _)| *known == name) {
                Some((_, known)) => *known = value,
                None => environment.push((name, value)),
            }
        }
        Layer {
            command: layer.command.or(wider.command),
            arguments: layer.arguments.or(wider.arguments),
            folder: layer.folder.or(wider.folder),
            environment,
            shell_executable: layer.shell_executable.or(wider.shell_executable),
            shell_arguments: layer.shell_arguments.or(wider.shell_arguments),
        }
    }
}

/// The task an object of `dependsOn` identifies: a script by `script` and `path`, or a task of the file by the
/// name Gitardo gives a task of its kind.
fn identified(named: &Value) -> Option<TaskReference> {
    let script = named.get("script").and_then(Value::as_str);
    match (named.get("type").and_then(Value::as_str), script) {
        (Some("npm"), Some(script)) => {
            let folder = named.get("path").and_then(Value::as_str).unwrap_or_default();
            let folder = folder.replace('\\', "/");
            Some(TaskReference::npm(folder.trim_matches('/'), script))
        }
        _ => name_of(named).map(|name| TaskReference::workspace(&name)),
    }
}

/// A word of a command: a string, or `{ "value", "quoting" }`.
fn word_of(value: &Value) -> Option<Word> {
    if let Some(text) = value.as_str() {
        return Some(Word::plain(text));
    }
    let quoting = match value.get("quoting").and_then(Value::as_str) {
        Some("escape") => Quoting::Escape,
        Some("strong") => Quoting::Strong,
        Some("weak") => Quoting::Weak,
        _ => Quoting::Plain,
    };
    Some(Word {
        value: value.get("value")?.as_str()?.to_owned(),
        quoting,
    })
}

/// How an entry runs: in a shell, as a process when it does not say, as VS Code takes it, or by something of VS Code.
fn kind_of(entry: &Value) -> TaskKind {
    match entry.get("type").and_then(Value::as_str) {
        Some("shell") => TaskKind::Shell,
        Some("process") | None => TaskKind::Process,
        Some(other) => TaskKind::Other(other.to_owned()),
    }
}

/// The name of an entry: its `label`, or `taskName` as older files say; without one, its command for a task Gitardo
/// runs, and `<type>: <script>` for the others, as VS Code names `npm: build`.
fn name_of(entry: &Value) -> Option<String> {
    let text = |name: &str| entry.get(name).and_then(Value::as_str).filter(|text| !text.is_empty());
    if let Some(label) = text("label").or_else(|| text("taskName")) {
        return Some(label.to_owned());
    }
    match kind_of(entry) {
        TaskKind::Shell | TaskKind::Process => entry.get("command").and_then(word_of).map(|word| word.value),
        TaskKind::Other(kind) => {
            let what = text("script").or_else(|| text("task")).or_else(|| text("command"));
            Some(what.map_or(kind.clone(), |what| format!("{kind}: {what}")))
        }
    }
}

/// The `group` of an entry: a name, or `{ "kind", "isDefault" }`.
fn category_of(entry: &Value) -> Option<TaskCategory> {
    let group = entry.get("group")?;
    let kind = group.as_str().or_else(|| group.get("kind")?.as_str())?;
    Some(TaskCategory {
        kind: kind.to_owned(),
        is_default: group.get("isDefault").and_then(Value::as_bool) == Some(true),
    })
}

/// What says a background task is ready: the `background` of its problem matcher, the first that has one. A matcher
/// named by a string, such as `$tsc-watch`, is one of VS Code's own, whose patterns Gitardo does not have.
fn background_of(entry: &Value) -> Background {
    let matchers = match entry.get("problemMatcher") {
        Some(Value::Array(items)) => items.iter().collect(),
        Some(one) => vec![one],
        None => Vec::new(),
    };
    let pattern = |background: &Value, name: &str| {
        let pattern = background.get(name)?;
        let text = pattern.as_str().or_else(|| pattern.get("regexp")?.as_str())?;
        Some(text.to_owned())
    };
    matchers
        .into_iter()
        .find_map(|matcher| matcher.get("background"))
        .map(|background| Background {
            begins_pattern: pattern(background, "beginsPattern"),
            ends_pattern: pattern(background, "endsPattern"),
        })
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;
