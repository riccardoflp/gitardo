//! The logo and the name of Gitardo at the top left of the window (`window.brand`): the logo in the square above the bar
//! of views, the name at the start of the header bar, over clones made for each test.

use platform::accesskit::Role;
use render::ThemeName;
use views::Intent;
use views::text::header;

use super::inspector::ask;
use super::pointer::{bounds, click_at, middle};
use super::side_column::{HEADER, column, header_button, in_header};
use super::support::{Window, has};
use super::tabs::{WIDE, three_clones};
use super::where_one_is::api_with_login;

/// The width of the bar of views, and of the square above it.
const BAR: f32 = 48.0;

/// A window too narrow for the name.
const NARROW: (u32, u32) = (700, 520);

#[test]
fn window_brand__the_logo_is_drawn_in_the_square_above_the_bar_of_views_in_each_theme() {
    let (_kept, _clones, state) = three_clones();
    for (theme, name) in [
        (ThemeName::Dark, "brand-header-dark"),
        (ThemeName::Light, "brand-header-light"),
        (ThemeName::HighContrastDark, "brand-header-high-contrast-dark"),
        (ThemeName::HighContrastLight, "brand-header-high-contrast-light"),
    ] {
        let mut window = Window::new(theme, WIDE);
        let (_, tree) = window.draw(&state, &[]);
        let logo = in_header(&tree, Role::Label, header::LOGO);
        assert!(logo.x >= 0.0 && logo.right() <= BAR, "in the square: {logo:?}");
        let timeline = bounds(&tree, Role::Button, "Timeline");
        assert!(
            timeline.y >= HEADER,
            "the bar of views is under the square: {timeline:?}"
        );
        window.assert_top_snapshot(name, 48);
    }
}

#[test]
fn window_brand__the_header_bar_says_gitardo_before_api_login_feature_login() {
    let (_kept, _clones, login, mut state) = api_with_login();
    ask(&mut state, Intent::ShowInTab(0, login));
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    let name = in_header(&tree, Role::Label, header::NAME);
    let api = header_button(&tree, "api");
    let place = header_button(&tree, "login");
    let branch = in_header(&tree, Role::Label, "feature/login");
    assert!(
        name.right() <= api.x && api.x < place.x && place.x < branch.x,
        "in that order"
    );
    assert!(
        (name.x - column(&tree).x).abs() <= 12.0,
        "the name starts where the side column does: {name:?}"
    );
}

#[test]
fn window_brand__in_a_window_700_pixels_wide_the_name_is_not_drawn_and_the_logo_is() {
    let (_kept, _clones, state) = three_clones();
    let mut window = Window::new(ThemeName::Dark, NARROW);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::Label, header::NAME), "no name");
    let logo = in_header(&tree, Role::Label, header::LOGO);
    let pixels = window.frame.to_rgba();
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "inside the window"
    )]
    let inked = (logo.y as usize..logo.bottom() as usize)
        .flat_map(|y| (logo.x as usize..logo.right() as usize).map(move |x| (y * NARROW.0 as usize + x) * 4))
        .filter(|at| pixels[*at..*at + 3] != [0x0E, 0x0F, 0x12])
        .count();
    assert!(inked > 20, "the logo is drawn: {inked} pixels");
    window.assert_top_snapshot("brand-header-narrow-dark", 48);
}

#[test]
fn window_brand__a_double_click_on_the_logo_or_on_the_name_asks_nothing_of_gitardo() {
    let (_kept, _clones, state) = three_clones();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let logo = in_header(&tree, Role::Label, header::LOGO);
    let name = in_header(&tree, Role::Label, header::NAME);
    for place in [logo, name] {
        let intents = click_at(&mut window, &state, middle(place), 2);
        assert_eq!(intents, [] as [Intent; 0], "{place:?}");
    }
}
