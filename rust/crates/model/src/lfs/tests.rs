#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::pointer;

/// The id of a content.
const OID: &str = "4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393";

/// The three lines `git lfs` writes for a content of so many bytes.
fn written(size: u64) -> String {
    format!("version https://git-lfs.github.com/spec/v1\noid sha256:{OID}\nsize {size}\n")
}

#[test]
fn lfs_pointer__the_three_lines_of_a_file_of_2000_bytes_give_its_id_and_size() {
    let found = pointer(written(2000).as_bytes()).unwrap();
    assert_eq!(found.oid, OID);
    assert_eq!(found.size, 2000);
    assert_eq!(found.others, Vec::<(String, String)>::new());
}

#[test]
fn lfs_pointer__crlf_another_order_or_an_id_of_63_digits_is_not_a_pointer() {
    assert_eq!(pointer(written(2000).replace('\n', "\r\n").as_bytes()), None);
    let reordered = format!("version https://git-lfs.github.com/spec/v1\nsize 2000\noid sha256:{OID}\n");
    assert_eq!(pointer(reordered.as_bytes()), None);
    let short = written(2000).replace(OID, &OID[1..]);
    assert_eq!(pointer(short.as_bytes()), None);
}

#[test]
fn lfs_pointer__text_that_starts_as_a_pointer_and_goes_past_1024_bytes_is_not_one() {
    let long = format!("{}x-padding {}\n", written(2000), "a".repeat(1100));
    assert_eq!(pointer(long.as_bytes()), None);
    assert_eq!(pointer(b""), None, "an empty file is an empty file");
}
