//! The places of each app, as the guide names them: the table of the names of the places of the window, the places of
//! the extension in VS Code, and the place of the cards of each area of the catalog.

use super::clean::clean;

/// The place of the cards of each file of the catalog, by the number of the file, in the words of the names of the
/// places of the window.
const AREAS: [(&str, &str); 20] = [
    ("01", "Repositories, in the side column"),
    ("02", "the timeline"),
    ("03", "the header, Search and Quick search"),
    ("04", "the inspector and the Commit page"),
    ("05", "the draft and the Changes page"),
    ("06", "the branch rail"),
    ("07", "the timeline"),
    ("08", "Kept aside, in the branch rail"),
    ("09", "the Conflicts page"),
    ("10", "the Compare page"),
    ("11", "the Files page"),
    ("12", "Kept aside, in the branch rail"),
    ("13", "Kept aside, in the branch rail"),
    ("14", "the timeline"),
    ("15", "the branch rail"),
    ("16", "the header and the bottom panel"),
    ("17", "the Diff and Merge pages"),
    ("18", "the Settings page"),
    ("19", "the bottom panel"),
    ("21", "the Files page"),
];

/// The place of the cards of a file of the catalog: `docs/features/06-branches-tags-remotes.md` is the branch rail.
pub fn of_area(file: &str) -> &'static str {
    let name = file.rsplit('/').next().unwrap_or_default();
    AREAS
        .iter()
        .find(|(number, _)| name.starts_with(number))
        .map_or("the window", |(_, place)| place)
}

/// The names of the places of the window, one a line, from the table of `docs/desktop/01-product.md`.
pub fn desktop(product: &str) -> String {
    let mut lines = String::new();
    let mut is_inside = false;
    for line in product.lines() {
        if line.starts_with('#') {
            is_inside = line == "### The names of the places";
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if let (true, Some(place), Some(what)) = (is_inside, cells.get(1), cells.get(2))
            && place.starts_with("**")
        {
            lines.push_str(&clean(&place.replace("**", "")));
            lines.push_str(": ");
            lines.push_str(&clean(what));
            lines.push('\n');
        }
    }
    lines
}

/// The places of the extension in VS Code, one a line, from **Where it is** of `docs/extension/01-product.md`.
pub fn extension(product: &str) -> String {
    let mut places: Vec<String> = Vec::new();
    let mut is_inside = false;
    for line in product.lines() {
        if line.starts_with("## ") {
            is_inside = line == "## Where it is";
            continue;
        }
        if !is_inside {
            continue;
        }
        if let Some(item) = line.strip_prefix("- ") {
            places.push(item.to_owned());
        } else if let (Some(continued), Some(last)) = (line.strip_prefix("  "), places.last_mut()) {
            last.push(' ');
            last.push_str(continued.trim());
        }
    }
    let mut lines = String::new();
    for place in &places {
        lines.push_str(&clean(&place.replace("**", "")));
        lines.push('\n');
    }
    lines
}
