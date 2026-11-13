//! The widgets that read their input themselves, driven over several frames: the input box, the list, the scroll area
//! and the splitter.

#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]
#![allow(
    clippy::single_range_in_vec_init,
    reason = "a selection is a list of ranges, also when it holds one"
)]

mod support;

use render::{Point, Rect, ThemeName};
use support::{Window, click, click_with, key, move_to, press, release, shortcut, typed, wheel};
use ui::{
    InputBox, InputBoxResponse, InputEvent, List, ListResponse, Modifiers, NamedKey, PointerButton, PointerButtonEvent,
    ScrollResponse, SelectionMode, Split, SplitResponse, Splitter, Ui, WidgetId,
};

/// An input box from 10 to 310 across, 10 to 36 down, showing what the view holds.
fn input_box(text: &str) -> impl Fn(&mut Ui<'_>) -> InputBoxResponse + '_ {
    move |ui| {
        let input = InputBox::new(text, "Branch name");
        ui.input_box("name", Rect::new(10.0, 10.0, 300.0, 26.0), &input)
    }
}

#[test]
fn an_input_box_takes_typing_selection_the_clipboard_and_undo() {
    let mut window = Window::new(ThemeName::Dark, (400, 100), 1);
    let _ = window.frame(&[], input_box(""));
    let (answer, _) = window.frame(&click(100.0, 20.0), input_box(""));
    assert!(answer.response.has_focus);
    let (answer, output) = window.frame(&[typed("fix "), typed("parser")], input_box(""));
    assert_eq!(answer.changed_text.as_deref(), Some("fix parser"));
    assert!(output.caret.is_some());
    let select_word = [key(NamedKey::ArrowLeft, Modifiers::CONTROL | Modifiers::SHIFT)];
    let _ = window.frame(&select_word, input_box("fix parser"));
    let (answer, output) = window.frame(&[shortcut('x', Modifiers::CONTROL)], input_box("fix parser"));
    assert_eq!(output.clipboard.as_deref(), Some("parser"));
    assert_eq!(answer.changed_text.as_deref(), Some("fix "));
    let paste = InputEvent::Paste(String::from("lexer\n"));
    let (answer, _) = window.frame(&[paste], input_box("fix "));
    assert_eq!(answer.changed_text.as_deref(), Some("fix lexer "));
    let (answer, _) = window.frame(&[shortcut('z', Modifiers::CONTROL)], input_box("fix lexer "));
    assert_eq!(answer.changed_text.as_deref(), Some("fix "));
    let (answer, _) = window.frame(&[key(NamedKey::Enter, Modifiers::NONE)], input_box("fix "));
    assert!(answer.is_submitted && answer.changed_text.is_none());
}

#[test]
fn a_double_click_selects_a_word_and_a_drag_selects_across() {
    let mut window = Window::new(ThemeName::Dark, (400, 100), 1);
    let _ = window.frame(&[], input_box("origin main"));
    let id = WidgetId::ROOT.with("name");
    let _ = window.frame(&click_with(30.0, 20.0, Modifiers::NONE, 2), input_box("origin main"));
    let (answer, output) = window.frame(&[shortcut('c', Modifiers::CONTROL)], input_box("origin main"));
    assert_eq!(output.clipboard.as_deref(), Some("origin"));
    assert!(answer.changed_text.is_none());
    let drag = [press(19.0, 20.0), move_to(300.0, 20.0), release(300.0, 20.0)];
    let _ = window.frame(&drag, input_box("origin main"));
    let (_, output) = window.frame(&[shortcut('c', Modifiers::CONTROL)], input_box("origin main"));
    assert_eq!(output.clipboard.as_deref(), Some("origin main"));
    assert_eq!(window.memory.input_text(id), Some("origin main"));
}

#[test]
fn a_view_that_changes_its_text_replaces_what_the_box_holds() {
    let mut window = Window::new(ThemeName::Dark, (400, 100), 1);
    let _ = window.frame(&[], input_box("first"));
    let _ = window.frame(&click(100.0, 20.0), input_box("first"));
    let _ = window.frame(&[typed("!")], input_box("first"));
    let id = WidgetId::ROOT.with("name");
    assert_eq!(window.memory.input_text(id), Some("first!"));
    let _ = window.frame(&[], input_box(""));
    assert_eq!(window.memory.input_text(id), Some(""));
}

/// A list of a million rows from 0 to 300 across and 0 to 220 down: ten rows of 22 pixels in sight.
fn list(ui: &mut Ui<'_>) -> ListResponse {
    let list = List {
        name: "Rows",
        row_count: 1_000_000,
        row_height: 22.0,
        selection_mode: SelectionMode::Multiple,
        lead: 0.0,
    };
    ui.list("rows", Rect::new(0.0, 0.0, 300.0, 220.0), &list, |_, _| {})
}

#[test]
fn a_list_selects_with_clicks_shift_and_ctrl_and_moves_with_the_keys() {
    let mut window = Window::new(ThemeName::Dark, (300, 220), 1);
    let _ = window.frame(&[], list);
    let (answer, _) = window.frame(&click(50.0, 50.0), list);
    assert_eq!(answer.changed_selection.unwrap().ranges(), &[2..3]);
    let (answer, _) = window.frame(&click_with(50.0, 100.0, Modifiers::SHIFT, 1), list);
    assert_eq!(answer.changed_selection.unwrap().ranges(), &[2..5]);
    let (answer, _) = window.frame(&click_with(50.0, 160.0, Modifiers::CONTROL, 1), list);
    assert_eq!(answer.changed_selection.unwrap().ranges(), &[2..5, 7..8]);
    let (answer, _) = window.frame(&[key(NamedKey::PageDown, Modifiers::NONE)], list);
    assert_eq!(answer.changed_selection.unwrap().ranges(), &[17..18]);
    assert_eq!(
        answer.visible_rows.start, 8,
        "the row of the keyboard is brought into sight"
    );
    let (answer, _) = window.frame(&[key(NamedKey::End, Modifiers::NONE)], list);
    assert_eq!(answer.visible_rows, 999_990..1_000_000);
    let (answer, _) = window.frame(&[key(NamedKey::Enter, Modifiers::NONE)], list);
    assert_eq!(answer.activated, Some(999_999));
    let (answer, _) = window.frame(&click_with(50.0, 210.0, Modifiers::NONE, 2), list);
    assert_eq!(answer.activated, Some(999_999));
    let secondary = PointerButtonEvent {
        button: PointerButton::Secondary,
        position: Point::new(50.0, 10.0),
        modifiers: Modifiers::NONE,
        click_count: 1,
    };
    let events = [
        InputEvent::PointerPressed(secondary),
        InputEvent::PointerReleased(secondary),
    ];
    let (answer, _) = window.frame(&events, list);
    assert_eq!(answer.context_row, Some(999_990));
    assert!(answer.response.secondary_clicked);
}

#[test]
fn the_wheel_glides_through_a_list_and_stops_at_its_ends() {
    let mut window = Window::new(ThemeName::Dark, (300, 220), 1);
    let _ = window.frame(&[], list);
    let (_, output) = window.frame(&[wheel(50.0, 50.0, 3.0)], list);
    assert!(output.next_frame.is_some(), "a glide asks for its next frame");
    window.wait(200);
    let (answer, output) = window.frame(&[], list);
    assert_eq!(answer.visible_rows.start, 3);
    assert_eq!(output.next_frame, None);
    let _ = window.frame(&[wheel(50.0, 50.0, -100.0)], list);
    window.wait(200);
    let (answer, _) = window.frame(&[], list);
    assert_eq!(answer.visible_rows.start, 0);
}

/// A scroll area of 200 by 100 over content of 400 by 1000.
fn scroll_area(ui: &mut Ui<'_>) -> ScrollResponse {
    ui.scroll_area("area", Rect::new(0.0, 0.0, 200.0, 100.0), (400.0, 1000.0), |_, _| {})
        .0
}

#[test]
fn a_scroll_area_scrolls_by_wheel_keys_and_its_bar_and_never_past_its_content() {
    let mut window = Window::new(ThemeName::Dark, (300, 200), 1);
    let _ = window.frame(&[], scroll_area);
    let _ = window.frame(&[wheel(50.0, 50.0, 1000.0)], scroll_area);
    window.wait(200);
    let (answer, _) = window.frame(&[], scroll_area);
    assert_eq!(answer.offset, (0.0, 900.0));
    let _ = window.frame(&click(50.0, 50.0), scroll_area);
    let _ = window.frame(&[key(NamedKey::Home, Modifiers::NONE)], scroll_area);
    window.wait(200);
    let (answer, _) = window.frame(&[], scroll_area);
    assert_eq!(answer.offset.1, 0.0);
    // A page is the height of the area less a row, so that the last row in sight stays in sight.
    let page = 100.0 - f64::from(window.theme.sizes.row_height);
    let (answer, _) = window.frame(&[key(NamedKey::PageDown, Modifiers::NONE)], scroll_area);
    assert!(answer.offset.1 > 0.0 && answer.offset.1 < page, "the glide has started");
    window.wait(200);
    let (answer, _) = window.frame(&[], scroll_area);
    assert_eq!(answer.offset.1, page);
    let drag = [
        move_to(195.0, 15.0),
        press(195.0, 15.0),
        move_to(195.0, 200.0),
        release(195.0, 200.0),
    ];
    let (_, output) = window.frame(&drag, scroll_area);
    assert_eq!(
        output.next_frame,
        Some(window.now),
        "the content moved by the bar is drawn in the next frame"
    );
    let (answer, _) = window.frame(&[], scroll_area);
    assert_eq!(
        answer.offset.1, 900.0,
        "dragging the thumb past the end stops at the end"
    );
    assert_eq!(answer.content.y, -900.0);
}

/// A splitter of two columns over 500 pixels, the first 200 wide, each at least 100.
fn splitter(ui: &mut Ui<'_>) -> SplitResponse {
    let splitter = Splitter {
        split: Split::Columns,
        default_size: 200.0,
        least_first: 100.0,
        least_second: 100.0,
        size: None,
        step: 0.0,
    };
    ui.splitter("split", Rect::new(0.0, 0.0, 501.0, 100.0), &splitter)
}

#[test]
fn a_splitter_moves_with_a_drag_within_its_least_sizes_and_a_double_click_resets_it() {
    let mut window = Window::new(ThemeName::Dark, (600, 100), 1);
    let _ = window.frame(&[], splitter);
    let drag = [move_to(200.0, 50.0), press(200.0, 50.0), move_to(260.0, 50.0)];
    let (answer, output) = window.frame(&drag, splitter);
    assert_eq!((answer.size, answer.is_changed), (260.0, true));
    assert_eq!(output.cursor, ui::Cursor::ResizeEastWest);
    let (answer, _) = window.frame(&[move_to(590.0, 50.0), release(590.0, 50.0)], splitter);
    assert_eq!((answer.size, answer.second.width), (400.0, 100.0));
    let _ = window.frame(
        &[
            move_to(400.0, 50.0),
            press(400.0, 50.0),
            move_to(0.0, 50.0),
            release(0.0, 50.0),
        ],
        splitter,
    );
    let (answer, _) = window.frame(&[], splitter);
    assert_eq!(answer.size, 100.0);
    let (answer, _) = window.frame(&click_with(100.0, 50.0, Modifiers::NONE, 2), splitter);
    assert_eq!(answer.size, 200.0);
}

#[test]
fn an_input_box_that_hides_what_is_typed_gives_the_text_to_its_view_and_nothing_to_the_clipboard() {
    let hidden = |text: &str| {
        let text = text.to_owned();
        move |ui: &mut Ui<'_>| {
            let input = InputBox::new(&text, "Token").hidden();
            ui.input_box("token", Rect::new(10.0, 10.0, 300.0, 26.0), &input)
        }
    };
    let mut window = Window::new(ThemeName::Dark, (400, 100), 1);
    let _ = window.frame(&[], hidden(""));
    let _ = window.frame(&click(100.0, 20.0), hidden(""));
    let (answer, _) = window.frame(&[typed("secret")], hidden(""));
    assert_eq!(answer.changed_text.as_deref(), Some("secret"));
    let (_, output) = window.frame(
        &[shortcut('a', Modifiers::CONTROL), shortcut('c', Modifiers::CONTROL)],
        hidden("secret"),
    );
    assert_eq!(
        output.clipboard, None,
        "nothing is copied from a box that hides what is typed"
    );
    let mut shown = Window::new(ThemeName::Dark, (400, 100), 1);
    let plain = |ui: &mut Ui<'_>| {
        let input = InputBox::new("secret", "Token");
        ui.input_box("token", Rect::new(10.0, 10.0, 300.0, 26.0), &input)
    };
    let _ = shown.frame(&[], plain);
    let mut dotted = Window::new(ThemeName::Dark, (400, 100), 1);
    let _ = dotted.frame(&[], hidden("secret"));
    assert_ne!(
        shown.frame.to_rgba(),
        dotted.frame.to_rgba(),
        "the dots are drawn in place of the text"
    );
}
