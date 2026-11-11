//! Rules R5.1 and R5.3: `unsafe` only in `crates/platform`, where every block says why it is sound; every other crate
//! forbids it.

use super::Workspace;
use crate::files::has_extension;
use crate::problem::Problem;
use crate::source::{code_only, words};

/// The one crate allowed to use `unsafe`.
const PLATFORM: &str = "crates/platform";

/// What each crate root of every other crate says.
const FORBID: &str = "#![forbid(unsafe_code)]";

/// The problems of `unsafe` in the workspace.
pub fn check(workspace: &Workspace) -> Vec<Problem> {
    let mut found = Vec::new();
    for member in &workspace.members {
        let is_platform = member.folder == PLATFORM;
        for (file, text) in workspace.rust_files(&member.folder) {
            let code = code_only(text);
            if is_platform {
                found.extend(undocumented_blocks(text, &code).into_iter().map(|line| {
                    let message = String::from("This `unsafe` block has no `// SAFETY:` comment above it.");
                    Problem::at_line("R5.3", &file.relative, line, message)
                }));
                continue;
            }
            if is_crate_root(&member.folder, &file.relative) && !code.contains(FORBID) {
                let message = format!("The crate root does not say `{FORBID}`.");
                found.push(Problem::in_file("R5.1", &file.relative, message));
            }
            for (line, _) in words(&code).into_iter().filter(|(_, word)| *word == "unsafe") {
                let message = format!("`unsafe` is allowed only in {PLATFORM}.");
                found.push(Problem::at_line("R5.1", &file.relative, line, message));
            }
        }
    }
    found
}

/// Whether a file is the root of a library or of a binary of the crate in `folder`.
fn is_crate_root(folder: &str, relative: &str) -> bool {
    let Some(inside) = relative.strip_prefix(folder).and_then(|rest| rest.strip_prefix('/')) else {
        return false;
    };
    inside == "src/lib.rs"
        || inside == "src/main.rs"
        || inside
            .strip_prefix("src/bin/")
            .is_some_and(|name| has_extension(name, "rs") && !name.contains('/'))
}

/// The lines of the `unsafe` blocks that have no `// SAFETY:` comment on the line or in the comments just above.
pub fn undocumented_blocks(text: &str, code: &str) -> Vec<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let mut found = Vec::new();
    for (index, line) in code.lines().enumerate() {
        if !opens_unsafe_block(line) {
            continue;
        }
        let same_line = lines.get(index).is_some_and(|raw| raw.contains("SAFETY:"));
        let above = lines
            .iter()
            .take(index)
            .rev()
            .map(|raw| raw.trim_start())
            .take_while(|raw| raw.starts_with("//"))
            .any(|raw| raw.contains("SAFETY:"));
        if !(same_line || above) {
            found.push(index + 1);
        }
    }
    found
}

/// Whether a line of code opens an `unsafe { … }` block, as opposed to declaring an `unsafe fn`, `impl` or `extern`.
fn opens_unsafe_block(line: &str) -> bool {
    line.match_indices("unsafe").any(|(start, _)| {
        let before = line.get(..start).and_then(|text| text.chars().last());
        let is_word_start = before.is_none_or(|previous| !(previous.is_alphanumeric() || previous == '_'));
        let after = line.get(start + "unsafe".len()..).unwrap_or_default();
        is_word_start && after.trim_start().starts_with('{')
    })
}

#[cfg(test)]
mod tests {
    use super::{is_crate_root, undocumented_blocks};
    use crate::source::code_only;

    #[test]
    fn wants_a_safety_comment_on_every_block() {
        let text = "unsafe fn a() {}\n\
            // SAFETY: the handle is ours.\n\
            let b = unsafe { call() };\n\
            let c = unsafe { call() };\n\
            let d = unsafe { call() }; // SAFETY: it is.\n\
            // Something else.\n\
            // SAFETY: the pointer lives as long as the window.\n\
            unsafe {\n\
            unsafe impl Send for X {}\n\
            let e = \"unsafe { }\";\n";
        assert_eq!(undocumented_blocks(text, &code_only(text)), vec![4]);
    }

    #[test]
    fn knows_the_roots_of_a_crate() {
        let cases = [
            ("crates/app/src/main.rs", true),
            ("crates/app/src/lib.rs", true),
            ("crates/app/src/bin/gitardo-cli.rs", true),
            ("crates/app/src/store.rs", false),
            ("crates/app/tests/window.rs", false),
            ("crates/application/src/lib.rs", false),
        ];
        for (relative, expected) in cases {
            assert_eq!(is_crate_root("crates/app", relative), expected, "{relative}");
        }
    }
}
