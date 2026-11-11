//! PNG images written by testkit are read back as they were, from memory and from files.

use testkit::png::{Image, decode, encode, read_file, write_file};
use testkit::{Error, Random, Repository};

#[test]
fn images_of_many_sizes_are_read_back_as_they_were_written() {
    let mut random = Random::from_environment(2003);
    let sizes = [
        (1, 1),
        (2, 3),
        (7, 5),
        (13, 17),
        (101, 3),
        (1, 300),
        (333, 211),
        (701, 401),
    ];
    for (width, height) in sizes {
        let mut image = Image::filled(width, height, [0, 0, 0, 0]);
        for pixel in &mut image.pixels {
            *pixel = random.next_u64().to_le_bytes()[0];
        }
        let bytes = encode(&image).unwrap();
        assert_eq!(decode(&bytes).unwrap(), image, "{width}×{height}");
    }
}

#[test]
fn an_image_written_to_a_file_is_read_back_from_it() {
    let repository = Repository::new();
    let path = repository.path().join("snapshots").join("graph-dark.png");
    let mut image = Image::filled(9, 4, [30, 30, 30, 255]);
    image.set_pixel(8, 3, [255, 128, 0, 200]);
    write_file(&path, &image).unwrap();
    assert_eq!(read_file(&path).unwrap(), image);
    assert!(matches!(
        read_file(repository.path().join("missing.png")),
        Err(Error::File { .. })
    ));
}

#[test]
fn an_image_that_cannot_be_written_is_an_error() {
    let empty = Image {
        width: 0,
        height: 4,
        pixels: vec![],
    };
    assert!(matches!(encode(&empty), Err(Error::InvalidSize { .. })));
    let short = Image {
        width: 2,
        height: 2,
        pixels: vec![0; 15],
    };
    assert!(matches!(
        encode(&short),
        Err(Error::PixelCount {
            expected: 16,
            actual: 15
        })
    ));
}

#[test]
fn the_difference_of_two_images_can_be_written_beside_them() {
    let reference = Image::filled(5, 5, [200, 200, 200, 255]);
    let mut actual = reference.clone();
    actual.set_pixel(2, 2, [0, 0, 0, 255]);
    let difference = actual.difference_from(&reference).unwrap();
    assert_eq!(difference.differing_pixels, 1);
    assert_eq!(decode(&encode(&difference.image).unwrap()).unwrap(), difference.image);
}

#[test]
fn the_random_source_takes_its_seed_from_the_environment_or_the_default() {
    let random = Random::from_environment(77);
    let expected = std::env::var("GITARDO_TEST_SEED").map_or(77, |seed| seed.parse().unwrap());
    assert_eq!(random.seed(), expected);
}
