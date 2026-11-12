//! How the files are laid out, kept between runs: as last chosen in the inspector (`details.show`), the list to start
//! with, and in the pages of commits (`commit-page.view`), the tree to start with.

use json::Value;
use views::details::FileLayout;

use super::member;

/// The layouts of the files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileLayouts {
    /// The layout of the files of the inspector.
    pub inspector: FileLayout,
    /// The layout of the files of the pages of commits opened later.
    pub page: FileLayout,
}

impl Default for FileLayouts {
    fn default() -> FileLayouts {
        FileLayouts {
            inspector: FileLayout::List,
            page: FileLayout::Tree,
        }
    }
}

/// The layouts, as written: `{"inspector": "list", "page": "tree"}`.
pub(super) fn layouts_value(layouts: FileLayouts) -> Value {
    Value::Object(vec![
        member("inspector", Value::String(name(layouts.inspector).to_owned())),
        member("page", Value::String(name(layouts.page).to_owned())),
    ])
}

/// The layouts read back; those of a first start for what is not there.
pub(super) fn file_layouts(value: &Value) -> FileLayouts {
    let first = FileLayouts::default();
    let read = |key: &str| value.get(key).and_then(Value::as_str).and_then(layout);
    FileLayouts {
        inspector: read("inspector").unwrap_or(first.inspector),
        page: read("page").unwrap_or(first.page),
    }
}

/// The name a layout is written with.
fn name(layout: FileLayout) -> &'static str {
    match layout {
        FileLayout::List => "list",
        FileLayout::Tree => "tree",
    }
}

/// A layout read back by its name.
fn layout(name: &str) -> Option<FileLayout> {
    match name {
        "list" => Some(FileLayout::List),
        "tree" => Some(FileLayout::Tree),
        _ => None,
    }
}
