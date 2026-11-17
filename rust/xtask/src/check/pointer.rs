//! Rule R10.6: a view declares no widget only while another is `hovered`. The pointer on the widget declared takes
//! `hovered` from the other, the next frame does not declare it, and it comes and goes under the pointer.
//!
//! The check reads the statement a `.hovered` is in, from there to its end, and refuses a call that declares a widget
//! in it: in the block of an `if`, on the right of `&&`, in the closure of a `then`. It does not follow a value kept in
//! a variable or handed to a function, which only the types would tell (Q730).

use super::Workspace;
use crate::problem::Problem;
use crate::source::code_only;

/// The crate of the views.
const VIEWS: &str = "crates/views";

/// What a view reads to ask whether a widget is the topmost one under the pointer.
const ASKED: &str = ".hovered";

/// The functions of the toolkit that declare a widget which takes the pointer, on the layer they are called in.
const DECLARING: [&str; 17] = [
    "button",
    "choice",
    "code_editor",
    "drag_source",
    "drag_source_carrying",
    "input_box",
    "interact",
    "interact_above",
    "interact_with_events",
    "list",
    "pick_list",
    "scroll_area",
    "searchable_list",
    "splitter",
    "splitter_across",
    "steered_list",
    "steered_searchable_list",
];

/// The widgets the views declare only while another is `hovered`.
pub fn check(workspace: &Workspace) -> Vec<Problem> {
    let mut found = Vec::new();
    for (file, text) in workspace.rust_files(VIEWS) {
        let code = code_only(text);
        found.extend(declared_while_hovered(&code).into_iter().map(|line| {
            let message = String::from(
                "A widget is declared only while another is `hovered`, and the pointer on it ends that: ask \
                 `contains_pointer`.",
            );
            Problem::at_line("R10.6", &file.relative, line, message)
        }));
    }
    found
}

/// The lines of some code that read `hovered` in a statement that goes on to declare a widget.
pub fn declared_while_hovered(code: &str) -> Vec<usize> {
    let mut found = Vec::new();
    for (start, _) in code.match_indices(ASKED) {
        let after = code.get(start + ASKED.len()..).unwrap_or_default();
        if is_question(after) && declares_widget(governed(after)) {
            let before = code.get(..start).unwrap_or_default();
            found.push(before.matches('\n').count() + 1);
        }
    }
    found
}

/// Whether what follows `.hovered` leaves it the question of a response: not a longer name, such as `hovered_since`,
/// and not a value compared or taken apart, as the line of a blame under the pointer is.
fn is_question(after: &str) -> bool {
    let is_longer = after
        .chars()
        .next()
        .is_some_and(|next| next.is_alphanumeric() || next == '_');
    let rest = after.trim_start();
    let is_compared = rest.starts_with('=') || rest.starts_with("!=");
    let is_taken_apart = rest.starts_with('.') && !rest.starts_with(".then");
    !(is_longer || is_compared || is_taken_apart)
}

/// What a question governs: the code after it to the end of its statement. A `;` or a `,` beside it ends it, and so
/// does the brace that closes the block around it; the block of an `if` or a `match` it is the condition of is part of
/// it, with the `else` after it.
fn governed(after: &str) -> &str {
    let mut depth = 0_usize;
    for (index, character) in after.char_indices() {
        let is_beside = depth == 0;
        match character {
            '(' | '[' | '{' => depth += 1,
            ';' | ',' | '}' if is_beside => return after.get(..index).unwrap_or_default(),
            // The question was inside these: the expression goes on after them.
            ')' | ']' if is_beside => {}
            ')' | ']' => depth -= 1,
            '}' => {
                depth -= 1;
                let rest = after.get(index + 1..).unwrap_or_default();
                if depth == 0 && !rest.trim_start().starts_with("else") {
                    return after.get(..=index).unwrap_or_default();
                }
            }
            _ => {}
        }
    }
    after
}

/// Whether some code calls a function of the toolkit that declares a widget, as a method: `ui.button(…)`.
fn declares_widget(code: &str) -> bool {
    code.match_indices('.').any(|(dot, _)| {
        let after = code.get(dot + 1..).unwrap_or_default();
        let name: String = after
            .chars()
            .take_while(|character| character.is_alphanumeric() || *character == '_')
            .collect();
        DECLARING.contains(&name.as_str()) && after.get(name.len()..).is_some_and(|rest| rest.starts_with('('))
    })
}

#[cfg(test)]
mod tests {
    use super::declared_while_hovered;

    #[test]
    fn refuses_a_widget_declared_in_the_statement_that_reads_hovered() {
        let cases = [
            (
                "let is_closed = (tab.is_shown || response.hovered)\n    && ui.scope(key, |ui| ui.button(key, rect, &button)).clicked;\n",
                vec![1],
            ),
            (
                "fill();\nif !entries.is_empty() && (response.hovered || is_open) {\n    if ui.button(key, rect, &button).clicked {\n        open();\n    }\n}\n",
                vec![2],
            ),
            (
                "let shown = response.hovered.then(|| ui.interact(id, rect, sense));\n",
                vec![1],
            ),
            (
                "if is_marked {\n    fill();\n} else if response.hovered {\n    light();\n} else {\n    ui.choice(key, rect, &choice);\n}\n",
                vec![3],
            ),
            (
                "let row = pointer.filter(|_| list.response.hovered).map(|point| ui.interact(id, at(point), sense));\n",
                vec![1],
            ),
        ];
        for (code, lines) in cases {
            assert_eq!(declared_while_hovered(code), lines, "{code}");
        }
    }

    #[test]
    fn leaves_what_a_widget_asks_of_itself_and_what_is_not_the_question_of_a_response() {
        let cases = [
            "if response.hovered {\n    ui.fill_rect(rect, color);\n}\nlet clicked = ui.button(key, rect, &button).clicked;\n",
            "let is_lit = response.hovered || response.pressed;\nlet clicked = ui.button(key, rect, &button).clicked;\n",
            "let look = Look { is_lit: response.hovered, is_clicked: ui.button(key, rect, &button).clicked };\n",
            "draw(ui, response.hovered, ui.button(key, rect, &button));\n",
            "if state.blame.hovered == Some(line) {\n    ui.button(key, rect, &button);\n}\n",
            "let since = ui.hovered_since(&response)?;\nui.button(key, rect, &button);\n",
            "if response.contains_pointer {\n    ui.button(key, rect, &button);\n}\n",
            "if response.hovered && !rests {\n    ui.request_frame_at(later);\n}\nui.interact(id, rect, sense);\n",
            "if response.hovered {\n    ui.set_cursor(cursor);\n    ui.tooltip(&response, words);\n}\n",
        ];
        for code in cases {
            assert_eq!(declared_while_hovered(code), Vec::<usize>::new(), "{code}");
        }
    }
}
