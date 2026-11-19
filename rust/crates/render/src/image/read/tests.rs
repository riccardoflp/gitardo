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

/// An image of one color, of a size.
fn filled(width: u32, height: u32, color: [u8; 4]) -> image::RgbaImage {
    image::RgbaImage::from_pixel(width, height, image::Rgba(color))
}

/// The bytes of an image of one color, in a format of the library.
fn encoded(width: u32, height: u32, format: image::ImageFormat) -> Vec<u8> {
    let picture = image::DynamicImage::ImageRgba8(filled(width, height, [200, 40, 40, 255]));
    let picture = if format == image::ImageFormat::Jpeg {
        image::DynamicImage::ImageRgb8(picture.into_rgb8())
    } else {
        picture
    };
    let mut bytes = std::io::Cursor::new(Vec::new());
    picture.write_to(&mut bytes, format).unwrap();
    bytes.into_inner()
}

#[test]
fn file_view_read_image__a_jpeg_a_gif_a_webp_a_bmp_and_an_ico_are_each_read_with_their_size() {
    for (format, ours, path) in [
        (image::ImageFormat::Jpeg, ImageFormat::Jpeg, "photo.jpg"),
        (image::ImageFormat::Gif, ImageFormat::Gif, "loop.gif"),
        (image::ImageFormat::WebP, ImageFormat::WebP, "photo.webp"),
        (image::ImageFormat::Bmp, ImageFormat::Bmp, "old.bmp"),
        (image::ImageFormat::Ico, ImageFormat::Ico, "favicon.ico"),
    ] {
        let bytes = encoded(24, 12, format);
        // Its first bytes tell its format, whatever its name.
        let image = read_image(&bytes, "picture.dat", 1.0).unwrap_or_else(|why| panic!("{path}: {why:?}"));
        assert_eq!(image.format, ours, "{path}");
        assert_eq!((image.width, image.height), (24, 12), "{path}");
        assert_eq!(image.weight, bytes.len() as u64);
        let [red, green, blue, opacity] = image.image.pixel(3, 3);
        assert!(
            red > 180 && green < 70 && blue < 70 && opacity == 255,
            "{path}: {red} {green} {blue}"
        );
    }
}

#[test]
fn file_view_read_image__an_animated_gif_gives_its_first_frame() {
    let mut bytes = Vec::new();
    {
        let mut encoder = image::codecs::gif::GifEncoder::new(&mut bytes);
        let frames = [[255, 0, 0, 255], [0, 0, 255, 255]].map(|color| image::Frame::new(filled(8, 8, color)));
        encoder.encode_frames(frames).unwrap();
    }
    let read = read_image(&bytes, "loop.gif", 1.0).unwrap();
    assert_eq!(read.image.pixel(4, 4), [255, 0, 0, 255]);
}

#[test]
fn file_view_read_image__an_ico_gives_its_largest_image() {
    let mut bytes = Vec::new();
    let frames: Vec<image::codecs::ico::IcoFrame<'_>> =
        [(16, [0, 0, 255, 255]), (48, [0, 255, 0, 255]), (32, [255, 0, 0, 255])]
            .into_iter()
            .map(|(side, color)| {
                image::codecs::ico::IcoFrame::as_png(
                    filled(side, side, color).as_raw(),
                    side,
                    side,
                    image::ExtendedColorType::Rgba8,
                )
                .unwrap()
            })
            .collect();
    image::codecs::ico::IcoEncoder::new(&mut bytes)
        .encode_images(&frames)
        .unwrap();
    let read = read_image(&bytes, "favicon.ico", 1.0).unwrap();
    assert_eq!((read.width, read.height), (48, 48));
    assert_eq!(read.image.pixel(10, 10), [0, 255, 0, 255]);
}

#[test]
fn file_view_read_image__a_jpeg_cut_short_cannot_be_read() {
    let bytes = encoded(24, 12, image::ImageFormat::Jpeg);
    assert_eq!(read_image(&bytes[..20], "photo.jpg", 1.0), Err(NotRead::Unreadable));
}
