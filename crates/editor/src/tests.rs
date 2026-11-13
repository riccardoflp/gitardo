use super::*;

/// A document of a text.
fn document(text: &str) -> Document {
    Document::from_bytes(text.as_bytes()).unwrap()
}

/// Does commands on a document and a cursor.
fn run(document: &mut Document, cursor: &mut Cursor, commands: &[Command]) {
    for command in commands {
        let _ = apply(document, cursor, command);
    }
}

/// Types a text one character at a time, as the keys do.
fn type_text(document: &mut Document, cursor: &mut Cursor, text: &str) {
    for character in text.chars() {
        let _ = apply(document, cursor, &Command::Type(character.to_string()));
    }
}

#[test]
fn the_bytes_of_a_file_come_back_as_they_were() {
    for bytes in [
        &b"one\r\ntwo\r\n"[..],
        b"one\ntwo",
        b"mixed\r\nlines\nhere\r\n",
        b"\xef\xbb\xbfwith a mark\n",
        b"",
        b"\n\n",
    ] {
        let buffer = Buffer::from_bytes(bytes).unwrap();
        assert_eq!(buffer.to_bytes(), bytes, "{:?}", String::from_utf8_lossy(bytes));
    }
    assert_eq!(Buffer::from_bytes(b"a\0b"), Err(NotText::Binary));
}

#[test]
fn lines_and_places_are_found_both_ways() {
    let buffer = Buffer::from_bytes("first\r\nsécond\nthird".as_bytes()).unwrap();
    assert_eq!(buffer.line_count(), 3);
    assert_eq!(buffer.line(1), "sécond");
    assert_eq!(buffer.text(), "first\nsécond\nthird");
    let position = Position { line: 1, column: 3 };
    assert_eq!(buffer.offset(position), 9);
    assert_eq!(buffer.position(9), position);
    assert_eq!(
        buffer.offset(Position { line: 1, column: 2 }),
        7,
        "inside é, it goes to its start"
    );
    assert_eq!(buffer.offset(Position { line: 9, column: 99 }), buffer.text().len());
    assert_eq!(buffer.slice_as_file(0..9), "first\r\nsé");
}

#[test]
fn lines_typed_take_the_line_breaks_of_the_file() {
    let mut text = document("one\r\ntwo\r\n");
    let mut cursor = Cursor::at(3);
    run(
        &mut text,
        &mut cursor,
        &[Command::NewLine, Command::Paste("a\nb".to_owned())],
    );
    assert_eq!(text.buffer().to_bytes(), b"one\r\na\r\nb\r\ntwo\r\n");
}

#[test]
fn three_typed_words_are_three_steps_and_undo_gives_back_the_saved_text() {
    let mut text = document("start\n");
    let mut cursor = Cursor::at(5);
    type_text(&mut text, &mut cursor, " one two three");
    assert_eq!(text.buffer().text(), "start one two three\n");
    assert!(text.is_unsaved());
    for _ in 0..3 {
        assert!(text.is_unsaved());
        run(&mut text, &mut cursor, &[Command::Undo]);
    }
    assert_eq!(text.buffer().text(), "start\n");
    assert!(!text.is_unsaved());
    assert_eq!(cursor, Cursor::at(5));
    run(&mut text, &mut cursor, &[Command::Redo]);
    assert_eq!(text.buffer().text(), "start one\n");
}

#[test]
fn moving_the_caret_ends_a_step_of_typing() {
    let mut text = document("");
    let mut cursor = Cursor::default();
    type_text(&mut text, &mut cursor, "ab");
    run(&mut text, &mut cursor, &[Command::Move(Movement::Left, false)]);
    type_text(&mut text, &mut cursor, "c");
    run(&mut text, &mut cursor, &[Command::Undo]);
    assert_eq!(text.buffer().text(), "ab");
}

#[test]
fn saving_and_undoing_past_it_leaves_the_text_unsaved() {
    let mut text = document("a");
    let mut cursor = Cursor::at(1);
    type_text(&mut text, &mut cursor, "b");
    text.mark_saved();
    assert!(!text.is_unsaved());
    run(&mut text, &mut cursor, &[Command::Undo]);
    assert!(text.is_unsaved());
    run(&mut text, &mut cursor, &[Command::Redo]);
    assert!(!text.is_unsaved());
}

#[test]
fn deleting_by_characters_and_words_and_the_selection() {
    let mut text = document("let value = 42;");
    let mut cursor = Cursor::at(9);
    run(&mut text, &mut cursor, &[Command::DeleteBackward(Extent::Word)]);
    assert_eq!(text.buffer().text(), "let  = 42;");
    run(&mut text, &mut cursor, &[Command::DeleteForward(Extent::Character)]);
    assert_eq!(text.buffer().text(), "let = 42;");
    run(&mut text, &mut cursor, &[Command::SelectAll, Command::DeleteSelection]);
    assert_eq!(text.buffer().text(), "");
}

#[test]
fn the_arrows_keep_the_column_across_shorter_lines() {
    let text = document("a long line\nab\nanother long line");
    let mut document = text;
    let mut cursor = Cursor::at(8);
    run(&mut document, &mut cursor, &[Command::Move(Movement::Down(1), false)]);
    assert_eq!(cursor.head, 14);
    run(&mut document, &mut cursor, &[Command::Move(Movement::Down(1), false)]);
    assert_eq!(cursor.head, 23);
    run(&mut document, &mut cursor, &[Command::Move(Movement::LineEnd, true)]);
    assert_eq!(cursor.range(), 23..32);
    run(&mut document, &mut cursor, &[Command::Move(Movement::Left, false)]);
    assert_eq!(cursor, Cursor::at(23));
}

#[test]
fn home_goes_to_the_text_of_the_line_then_to_its_start_and_words_are_selected() {
    let mut text = document("    indented word");
    let mut cursor = Cursor::at(12);
    run(&mut text, &mut cursor, &[Command::Move(Movement::LineStart, false)]);
    assert_eq!(cursor.head, 4);
    run(&mut text, &mut cursor, &[Command::Move(Movement::LineStart, false)]);
    assert_eq!(cursor.head, 0);
    run(
        &mut text,
        &mut cursor,
        &[Command::SelectWord(Position { line: 0, column: 15 })],
    );
    assert_eq!(cursor.range(), 13..17);
    run(&mut text, &mut cursor, &[Command::Move(Movement::WordLeft, false)]);
    assert_eq!(cursor.head, 13);
}

#[test]
fn the_cursor_of_another_page_follows_the_text() {
    let mut text = document("alpha beta");
    let mut mine = Cursor::at(0);
    let mut theirs = Cursor {
        anchor: 6,
        head: 10,
        goal: None,
    };
    let changes = apply(&mut text, &mut mine, &Command::Type("X".to_owned()));
    for change in &changes {
        theirs.follow(change);
    }
    assert_eq!(theirs.range(), 7..11);
    assert_eq!(&text.buffer().text()[theirs.range()], "beta");
}

#[test]
fn a_change_says_which_lines_it_replaced() {
    let mut text = document("one\ntwo\nthree");
    let mut cursor = Cursor {
        anchor: 2,
        head: 9,
        goal: None,
    };
    let changes = apply(&mut text, &mut cursor, &Command::Paste("X".to_owned()));
    assert_eq!(
        changes.first().map(|change| change.lines),
        Some(LinesChanged {
            start: 0,
            removed: 3,
            added: 1
        })
    );
    assert_eq!(text.buffer().text(), "onXhree");
}
