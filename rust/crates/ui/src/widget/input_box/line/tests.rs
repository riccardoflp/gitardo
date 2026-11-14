use super::*;

#[test]
fn typing_replaces_the_selection_and_moves_the_caret() {
    let mut line = EditableLine::new("fix parser");
    line.select(4, 10);
    assert_eq!(line.selected_text(), "parser");
    assert!(line.insert("lexer"));
    assert_eq!((line.text(), line.caret(), line.selection()), ("fix lexer", 9, 9..9));
    assert!(line.insert("\r\nnow"));
    assert_eq!(line.text(), "fix lexer now");
}

#[test]
fn shift_with_the_arrows_extends_and_plain_arrows_collapse() {
    let mut line = EditableLine::new("one two three");
    line.move_caret(Movement::Home, Selecting::Move);
    line.move_caret(Movement::WordRight, Selecting::Extend);
    line.move_caret(Movement::WordRight, Selecting::Extend);
    assert_eq!(line.selected_text(), "one two ");
    line.move_caret(Movement::Left, Selecting::Move);
    assert_eq!((line.caret(), line.selection()), (0, 0..0));
    line.move_caret(Movement::End, Selecting::Extend);
    line.move_caret(Movement::WordLeft, Selecting::Extend);
    assert_eq!(line.selected_text(), "one two ");
    line.move_caret(Movement::Right, Selecting::Move);
    assert_eq!(line.caret(), 8);
}

#[test]
fn backspace_and_delete_remove_characters_words_or_the_selection() {
    let mut line = EditableLine::new("café au lait");
    assert!(line.delete_backward(Extent::Word));
    assert_eq!(line.text(), "café au ");
    assert!(line.delete_backward(Extent::Character));
    line.move_caret(Movement::Home, Selecting::Move);
    assert!(line.delete_forward(Extent::Word));
    assert_eq!(line.text(), "au");
    line.select_all();
    assert_eq!(line.cut(), Some(String::from("au")));
    assert_eq!(line.cut(), None);
    assert_eq!(line.text(), "");
    assert!(!line.delete_backward(Extent::Character));
}

#[test]
fn a_word_typed_is_undone_in_one_step_and_redone() {
    let mut line = EditableLine::new("");
    for character in ["f", "i", "x", " ", "i", "t"] {
        let _ = line.insert(character);
    }
    assert_eq!(line.text(), "fix it");
    assert!(line.undo());
    assert_eq!(line.text(), "fix ");
    assert!(line.undo());
    assert_eq!(line.text(), "");
    assert!(!line.undo());
    assert!(line.redo());
    assert!(line.redo());
    assert_eq!((line.text(), line.caret()), ("fix it", 6));
    assert!(!line.redo());
    let _ = line.delete_backward(Extent::Character);
    let _ = line.delete_backward(Extent::Character);
    assert!(line.undo());
    assert_eq!(line.text(), "fix it");
}

#[test]
fn an_edit_after_undo_forgets_what_was_undone() {
    let mut line = EditableLine::new("a");
    let _ = line.insert("b");
    assert!(line.undo());
    let _ = line.insert("c");
    assert!(!line.redo());
    assert_eq!(line.text(), "ac");
}

#[test]
fn a_double_click_selects_a_word_and_positions_never_split_a_character() {
    let mut line = EditableLine::new("origin/main");
    line.select_word_at(2);
    assert_eq!(line.selected_text(), "origin");
    let mut line = EditableLine::new("é");
    line.select(1, 1);
    assert_eq!(line.caret(), 0);
}
