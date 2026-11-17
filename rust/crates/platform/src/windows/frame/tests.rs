#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::*;

#[test]
fn window_title_bar__the_system_is_told_the_caption_the_maximize_button_and_the_edges() {
    let cases = [
        (FramePart::Client, user32::HTCLIENT),
        (FramePart::Caption, user32::HTCAPTION),
        (FramePart::Minimize, user32::HTCLIENT),
        (FramePart::Maximize, user32::HTMAXBUTTON),
        (FramePart::Close, user32::HTCLIENT),
        (FramePart::Edge(ResizeEdge::Left), user32::HTLEFT),
        (FramePart::Edge(ResizeEdge::Right), user32::HTRIGHT),
        (FramePart::Edge(ResizeEdge::Top), user32::HTTOP),
        (FramePart::Edge(ResizeEdge::Bottom), user32::HTBOTTOM),
        (FramePart::Edge(ResizeEdge::TopLeft), user32::HTTOPLEFT),
        (FramePart::Edge(ResizeEdge::TopRight), user32::HTTOPRIGHT),
        (FramePart::Edge(ResizeEdge::BottomLeft), user32::HTBOTTOMLEFT),
        (FramePart::Edge(ResizeEdge::BottomRight), user32::HTBOTTOMRIGHT),
    ];
    for (part, code) in cases {
        assert_eq!(hit_code(part), code, "{part:?}");
    }
}

#[test]
fn window_title_bar__alt_space_opens_the_system_menu_of_the_window() {
    assert!(opens_window_menu(usize::from(b' ')));
    assert!(!opens_window_menu(usize::from(b'f')));
}

#[test]
fn reads_the_point_of_the_screen_of_a_message_with_its_signs() {
    assert_eq!(screen_point(0x0014_000A), (10, 20));
    // A screen left of and above the main one gives negative coordinates.
    assert_eq!(
        screen_point(isize::try_from(0xFFF6_FFEC_u32.cast_signed()).unwrap()),
        (-20, -10)
    );
}
