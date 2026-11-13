//! `submodule.copy-path` and `submodule.copy-url`: the folder or the URL of a submodule, on the clipboard.

use model::contract::{Answers, Context, Danger, FollowUp, Outcome, Plan, Text};

use super::{nothing_to_undo, refusal, target_submodule};

/// Puts the folder of the submodule, from the root of the repository, on the clipboard: no steps.
///
/// Feature: submodule.copy-path
#[must_use]
pub fn copy_path(context: &Context, _answers: &Answers) -> Outcome {
    let submodule = match target_submodule(context) {
        Ok(submodule) => submodule,
        Err(refusal) => return *refusal,
    };
    copy(
        format!("Copy the path of {}", submodule.folder),
        submodule.folder.clone(),
    )
}

/// Puts the URL `.gitmodules` gives the submodule on the clipboard, as it is written there: no steps.
///
/// Feature: submodule.copy-url
#[must_use]
pub fn copy_url(context: &Context, _answers: &Answers) -> Outcome {
    let submodule = match target_submodule(context) {
        Ok(submodule) => submodule,
        Err(refusal) => return *refusal,
    };
    let Some(url) = &submodule.url else {
        return *refusal(&format!("{} has no URL in .gitmodules.", submodule.folder));
    };
    copy(format!("Copy the URL of {}", submodule.folder), url.clone())
}

/// A plan that puts some text on the clipboard and runs nothing.
fn copy(title: String, text: String) -> Outcome {
    Outcome::Run(Plan {
        title: Text::from(title),
        steps: Vec::new(),
        danger: Danger::None,
        undo: nothing_to_undo(),
        after: vec![FollowUp::CopyToClipboard(text)],
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answers, FollowUp};

    use super::{copy_path, copy_url};
    use crate::submodule::testing::{lib, nested, on};
    use crate::testing::plan;

    #[test]
    fn submodule_copy_path__on_vendor_a_b_the_clipboard_holds_vendor_a_b() {
        let plan = plan(copy_path(&on(nested()), &Answers::new()));
        assert_eq!(plan.after, [FollowUp::CopyToClipboard(String::from("vendor/a/b"))]);
        assert_eq!(plan.steps, []);
    }

    #[test]
    fn submodule_copy_url__on_lib_the_clipboard_holds_its_url_as_gitmodules_writes_it() {
        let plan = plan(copy_url(&on(lib()), &Answers::new()));
        assert_eq!(plan.after, [FollowUp::CopyToClipboard(String::from("../lib.git"))]);
    }
}
