//! The three files the settings of new worktrees are in, read through a query and edited in place: the repository's
//! `.gitardo/settings.json`, the user's `settings.json`, and VS Code's `.vscode/settings.json` of the repository.

use std::path::PathBuf;

use json::{Key, Value};
use model::contract::{Answer, AnswerKey, Answers, Context, Outcome, Query, Refusal, Step};
use model::worktree_setup::{
    Places, SettingSource, SettingsInput, SettingsPlace, SetupPatterns, SetupSettings, settings_in_effect,
};

/// The key the settings files come back under.
pub const SETTINGS_FILES: AnswerKey = AnswerKey("worktree-setup-files");

/// `worktrees.linkFolders`.
pub const LINK_FOLDERS: &str = "worktrees.linkFolders";
/// `worktrees.copyFiles`.
pub const COPY_FILES: &str = "worktrees.copyFiles";
/// `worktrees.setupCommands`.
pub const SETUP_COMMANDS: &str = "worktrees.setupCommands";
/// VS Code's `git.worktreeSymlinkFolders`.
pub const VS_CODE_LINK_FOLDERS: &str = "git.worktreeSymlinkFolders";
/// VS Code's `git.worktreeIncludeFiles`.
pub const VS_CODE_COPY_FILES: &str = "git.worktreeIncludeFiles";

/// A settings file: where it is, and its text when it is there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsFile {
    /// The file.
    pub path: PathBuf,
    /// Its text; none when it is not there.
    pub text: Option<String>,
}

impl SettingsFile {
    /// The list a key sets, when the key is there with a list; a file that is not JSON with comments sets nothing.
    #[must_use]
    pub fn list(&self, key: &str) -> Option<Vec<String>> {
        let value = json::parse_with_comments(self.text.as_deref()?).ok()?;
        let items = value.get(key)?.as_array()?;
        Some(items.iter().filter_map(Value::as_str).map(str::to_owned).collect())
    }

    /// Whether the file sets a key.
    #[must_use]
    pub fn has(&self, key: &str) -> bool {
        self.text
            .as_deref()
            .and_then(|text| json::parse_with_comments(text).ok())
            .is_some_and(|value| value.get(key).is_some())
    }
}

/// The three files of the settings of new worktrees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsFiles {
    /// The repository's `.gitardo/settings.json`.
    pub repository: SettingsFile,
    /// The user's file; none when the system does not say where it is.
    pub user: Option<SettingsFile>,
    /// VS Code's `.vscode/settings.json` of the repository.
    pub vs_code: SettingsFile,
}

impl SettingsFiles {
    /// The file of a place, when it has one.
    #[must_use]
    pub fn of(&self, place: SettingsPlace) -> Option<&SettingsFile> {
        match place {
            SettingsPlace::Repository => Some(&self.repository),
            SettingsPlace::User => self.user.as_ref(),
        }
    }

    /// What new worktrees get, as set, with where each comes from.
    #[must_use]
    pub fn in_effect(&self) -> SetupSettings {
        fn places((repository, user): &(Option<Vec<String>>, Option<Vec<String>>)) -> Places<'_> {
            Places {
                repository: repository.as_deref(),
                user: user.as_deref(),
            }
        }
        let user = |key: &str| self.user.as_ref().and_then(|file| file.list(key));
        let lists = [LINK_FOLDERS, COPY_FILES, SETUP_COMMANDS].map(|key| (self.repository.list(key), user(key)));
        let vs_code = [VS_CODE_LINK_FOLDERS, VS_CODE_COPY_FILES].map(|key| self.vs_code.list(key));
        let [link, copy, commands] = &lists;
        let [vs_code_link, vs_code_copy] = &vs_code;
        settings_in_effect(&SettingsInput {
            link_folders: places(link),
            copy_files: places(copy),
            setup_commands: places(commands),
            vscode_link_folders: vs_code_link.as_deref(),
            vscode_copy_files: vs_code_copy.as_deref(),
        })
    }
}

/// The patterns in effect.
#[must_use]
pub fn patterns_of(settings: &SetupSettings) -> SetupPatterns {
    SetupPatterns {
        link: settings.link_folders.values.clone(),
        copy: settings.copy_files.values.clone(),
    }
}

/// What is said where a feature would write the settings files while the host gives the settings.
pub const SETTINGS_OF_THE_HOST: &str = "Inside VS Code, what new worktrees get is in VS Code's settings: Gitardo changes them there, not in a file of its own.";

/// What new worktrees get: what the host gives, inside VS Code, or else what the settings files set, read first.
///
/// # Errors
///
/// The read of the files, while they are not read yet.
pub fn settings_given(context: &Context, answers: &Answers) -> Result<SetupSettings, Box<Outcome>> {
    match &context.settings.worktrees.setup {
        Some(given) => Ok(given.clone()),
        None => Ok(read_files(context, answers)?.in_effect()),
    }
}

/// The settings files, or the outcome that reads them first. While the host gives the settings, no file is read, and
/// a feature that would write one is refused: the host writes its own.
///
/// # Errors
///
/// The read of the files, while they are not read yet; the refusal while the host gives the settings.
pub fn read_files(context: &Context, answers: &Answers) -> Result<SettingsFiles, Box<Outcome>> {
    if context.settings.worktrees.setup.is_some() {
        return Err(Box::new(Outcome::Refuse(Refusal::saying(SETTINGS_OF_THE_HOST))));
    }
    let folder = &context.repository.folder;
    let repository = folder.join(".gitardo").join("settings.json");
    let vs_code = folder.join(".vscode").join("settings.json");
    let user = context.settings.user_settings_file.clone();
    let mut paths = vec![repository.clone(), vs_code.clone()];
    paths.extend(user.clone());
    let Some(Answer::Files(texts)) = answers.get(SETTINGS_FILES) else {
        return Err(Box::new(Outcome::Read(SETTINGS_FILES, Query::FileTexts { paths })));
    };
    let text = |index: usize| {
        texts
            .get(index)
            .cloned()
            .flatten()
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    };
    Ok(SettingsFiles {
        repository: SettingsFile {
            path: repository,
            text: text(0),
        },
        vs_code: SettingsFile {
            path: vs_code,
            text: text(1),
        },
        user: user.map(|path| SettingsFile { path, text: text(2) }),
    })
}

/// A list as JSON.
#[must_use]
pub fn list_value(values: &[String]) -> Value {
    Value::Array(values.iter().cloned().map(Value::String).collect())
}

/// The text of a file with keys set, or taken out for none, the rest of it, comments included, as it was.
///
/// # Errors
///
/// The refusal that says the file is not JSON with comments.
pub fn edited(file: &SettingsFile, changes: &[(&str, Option<Value>)]) -> Result<String, Refusal> {
    let mut text = file.text.clone().unwrap_or_default();
    for (key, value) in changes {
        let path = [Key::Member((*key).to_owned())];
        let edit = match value {
            Some(value) => json::set_value(&text, &path, value),
            None => json::remove_value(&text, &path),
        };
        text = edit.map_err(|error| {
            Refusal::saying(&format!(
                "{} is not JSON with comments, so it was not changed: {error}",
                file.path.display()
            ))
        })?;
    }
    Ok(text)
}

/// The step that writes a file with its new text, none when the text is what the file has.
#[must_use]
pub fn write_step(file: &SettingsFile, text: String) -> Option<Step> {
    (file.text.as_deref() != Some(text.as_str())).then(|| Step::Write {
        path: file.path.clone(),
        contents: text.into_bytes(),
    })
}

/// Where a list in effect is written: where it is set, or the repository when it comes from VS Code or nothing.
#[must_use]
pub fn place_of(source: SettingSource) -> SettingsPlace {
    match source {
        SettingSource::User => SettingsPlace::User,
        SettingSource::Repository | SettingSource::VsCode | SettingSource::Nothing => SettingsPlace::Repository,
    }
}

/// The words of a place, for **Saved in the settings of the repository**.
#[must_use]
pub fn place_words(place: SettingsPlace) -> &'static str {
    match place {
        SettingsPlace::Repository => "the repository",
        SettingsPlace::User => "the user",
    }
}

/// What is said when the user's settings file has no place.
pub const NO_USER_FILE: &str = "The system does not say where your settings are, so they were not changed.";
