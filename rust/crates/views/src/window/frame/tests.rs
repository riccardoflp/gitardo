#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::*;

/// A window of 1200 by 800 logical pixels whose header bar is 44 high, with **Search** and **Sync** in it.
fn window_frame(title_bar: TitleBar, is_maximized: bool) -> (WindowSize, FrameState) {
    let size = WindowSize {
        width: 1200.0,
        height: 800.0,
    };
    let frame = FrameState {
        caption: CaptionState {
            title_bar,
            is_maximized,
        },
        caption_height: 44.0,
        controls: vec![
            // The repository in the header, Search and Sync.
            Rect::new(110.0, 8.0, 60.0, 28.0),
            Rect::new(700.0, 8.0, 160.0, 28.0),
            Rect::new(880.0, 8.0, 90.0, 28.0),
        ],
    };
    (size, frame)
}

#[test]
fn window_title_bar__hit_says_caption_for_a_free_place_of_the_header_bar_the_logo_and_the_name() {
    let (size, frame) = window_frame(TitleBar::Drawn, false);
    // A free place between the repository and Search, the logo above the bar of views, and the name before the
    // repository.
    for place in [Point::new(400.0, 22.0), Point::new(24.0, 22.0), Point::new(80.0, 22.0)] {
        assert_eq!(hit(place, size, &frame), Hit::Caption, "{place:?}");
    }
    // Under the header bar is the inside of the window.
    assert_eq!(hit(Point::new(400.0, 60.0), size, &frame), Hit::Client);
}

#[test]
fn window_title_bar__hit_says_client_for_search_sync_and_the_repository_in_the_header() {
    let (size, frame) = window_frame(TitleBar::Drawn, false);
    for place in [
        Point::new(120.0, 20.0),
        Point::new(750.0, 20.0),
        Point::new(900.0, 20.0),
    ] {
        assert_eq!(hit(place, size, &frame), Hit::Client, "{place:?}");
    }
}

#[test]
fn window_title_bar__hit_says_edge_and_corner_near_the_edges_and_client_while_maximized() {
    let (size, frame) = window_frame(TitleBar::Drawn, false);
    let left = Point::new(3.0, 400.0);
    let bottom_right = Point::new(1197.0, 797.0);
    assert_eq!(hit(left, size, &frame), Hit::Edge(Edge::Left));
    assert_eq!(hit(bottom_right, size, &frame), Hit::Corner(Corner::BottomRight));
    let cases = [
        (Point::new(600.0, 2.0), Hit::Edge(Edge::Top)),
        (Point::new(1198.0, 400.0), Hit::Edge(Edge::Right)),
        (Point::new(600.0, 799.0), Hit::Edge(Edge::Bottom)),
        (Point::new(1.0, 1.0), Hit::Corner(Corner::TopLeft)),
        (Point::new(1199.0, 1.0), Hit::Corner(Corner::TopRight)),
        (Point::new(2.0, 798.0), Hit::Corner(Corner::BottomLeft)),
        (Point::new(7.0, 400.0), Hit::Client),
    ];
    for (place, expected) in cases {
        assert_eq!(hit(place, size, &frame), expected, "{place:?}");
    }
    let (size, maximized) = window_frame(TitleBar::Drawn, true);
    assert_eq!(hit(left, size, &maximized), Hit::Client);
    assert_eq!(hit(bottom_right, size, &maximized), Hit::Client);
}

#[test]
fn window_caption_buttons__hit_says_minimize_maximize_and_close_at_the_right_end_reaching_the_top_while_maximized() {
    let (size, maximized) = window_frame(TitleBar::Drawn, true);
    let cases = [
        (Point::new(1080.0, 20.0), Hit::Minimize),
        (Point::new(1110.0, 0.0), Hit::Maximize),
        (Point::new(1199.5, 0.0), Hit::Close),
        (Point::new(1199.5, 43.0), Hit::Close),
        (Point::new(1199.5, 44.0), Hit::Client),
    ];
    for (place, expected) in cases {
        assert_eq!(hit(place, size, &maximized), expected, "{place:?}");
    }
}

#[test]
fn hit_says_client_everywhere_when_the_system_draws_the_title_bar_and_has_no_buttons_on_macos() {
    let (size, system) = window_frame(TitleBar::System, false);
    assert_eq!(hit(Point::new(400.0, 22.0), size, &system), Hit::Client);
    assert_eq!(hit(Point::new(3.0, 400.0), size, &system), Hit::Client);
    let (size, transparent) = window_frame(TitleBar::Transparent, false);
    assert_eq!(hit(Point::new(400.0, 22.0), size, &transparent), Hit::Caption);
    assert_eq!(hit(Point::new(1199.0, 20.0), size, &transparent), Hit::Caption);
    assert_eq!(hit(Point::new(3.0, 400.0), size, &transparent), Hit::Client);
}
