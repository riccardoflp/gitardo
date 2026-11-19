#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::*;
use testkit::png;

/// The bytes of a PNG of one color.
fn png_bytes(width: u32, height: u32) -> Vec<u8> {
    png::encode(&png::Image::filled(width, height, [10, 120, 200, 255])).unwrap()
}

#[test]
fn file_view_read_image__the_bytes_of_a_png_of_32_by_16_give_its_pixels_format_and_weight() {
    let bytes = png_bytes(32, 16);
    let read = read_image(&bytes, "media/logo.png", 2.0).unwrap();
    assert_eq!((read.width, read.height), (32, 16));
    assert_eq!((read.image.width(), read.image.height()), (32, 16));
    assert_eq!(read.format, ImageFormat::Png);
    assert_eq!(read.weight, bytes.len() as u64);
    assert_eq!(read.image.pixel(5, 5), [10, 120, 200, 255]);
}

#[test]
fn file_view_read_image__the_same_png_named_logo_dat_is_read_as_a_png() {
    let read = read_image(&png_bytes(32, 16), "logo.dat", 1.0).unwrap();
    assert_eq!(read.format, ImageFormat::Png);
}

#[test]
fn file_view_read_image__an_svg_of_10_by_20_at_a_scale_of_2_gives_20_by_40_pixels_and_a_size_of_10_by_20() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="20"><rect width="10" height="20" fill="#00ff00"/></svg>"##;
    let read = read_image(svg, "icons/Check.SVG", 2.0).unwrap();
    assert_eq!(read.format, ImageFormat::Svg);
    assert_eq!((read.width, read.height), (10, 20));
    assert_eq!((read.image.width(), read.image.height()), (20, 40));
}

#[test]
fn file_view_read_image__an_svg_that_names_an_image_and_a_font_not_of_the_program_draws_neither_and_reads_no_file() {
    let folder = std::env::temp_dir().join(format!("gitardo-read-image-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("secret.png"), png_bytes(4, 4)).unwrap();
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="20" height="10">
        <image x="0" y="0" width="20" height="10" xlink:href="{}"/>
        <text x="0" y="8" font-family="Some Font Of The System Only" font-size="8" fill="none">a</text></svg>"#,
        folder.join("secret.png").display()
    );
    let read = read_image(svg.as_bytes(), "a.svg", 1.0).unwrap();
    assert!((0..20).all(|x| (0..10).all(|y| read.image.pixel(x, y)[3] == 0)));
    let _removed = std::fs::remove_dir_all(&folder);
}

#[test]
fn file_view_read_image__a_png_cut_short_could_not_be_read_and_one_of_10000_by_10000_is_too_large() {
    let bytes = png_bytes(32, 16);
    assert_eq!(read_image(&bytes[..40], "a.png", 1.0), Err(NotRead::Unreadable));
    let mut large = bytes;
    large[16..20].copy_from_slice(&10_000u32.to_be_bytes());
    large[20..24].copy_from_slice(&10_000u32.to_be_bytes());
    assert_eq!(read_image(&large, "a.png", 1.0), Err(NotRead::TooLarge));
}

#[test]
fn file_view_read_image__notes_txt_is_not_an_image_and_an_svg_cut_short_says_what_is_wrong() {
    assert_eq!(read_image(b"Some notes.\n", "notes.txt", 1.0), Err(NotRead::NotImage));
    assert_eq!(image_format(b"Some notes.\n", "notes.txt"), None);
    let Err(NotRead::SvgError(said)) = read_image(b"<svg", "a.svg", 1.0) else {
        panic!("an SVG cut short is said to have an error");
    };
    assert!(said.len() > 3, "{said}");
}
