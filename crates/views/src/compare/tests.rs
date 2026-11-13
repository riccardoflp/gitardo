#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::compare::CompareMode;
use ui::ButtonKind;

use super::narrowed_path;
use super::parts::mode_button_kind;

#[test]
fn compare_narrow_to_path__dot_slash_src_slash_narrows_to_src() {
    assert_eq!(narrowed_path("./src/"), Some(String::from("src")));
    assert_eq!(narrowed_path("  src\\app\\  "), Some(String::from("src/app")));
    assert_eq!(narrowed_path("/docs"), Some(String::from("docs")));
}

#[test]
fn compare_narrow_to_path__nothing_or_a_dot_narrows_to_nothing() {
    assert_eq!(narrowed_path(""), None);
    assert_eq!(narrowed_path(" . "), None);
    assert_eq!(narrowed_path("./"), None);
}

#[test]
fn compare_set_mode__the_button_of_the_mode_shown_is_pressed_in() {
    let since = CompareMode::SinceDiverged;
    let everything = CompareMode::Everything;
    assert_eq!(mode_button_kind(since, since), ButtonKind::Secondary);
    assert_eq!(mode_button_kind(since, everything), ButtonKind::Outline);
    assert_eq!(mode_button_kind(everything, everything), ButtonKind::Secondary);
    assert_eq!(mode_button_kind(everything, since), ButtonKind::Outline);
}
