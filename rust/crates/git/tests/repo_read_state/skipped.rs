//! `changes.read-skipped` on throwaway repositories: the files marked skip-worktree or assume-unchanged, as the index
//! has them, and those of a sparse checkout left out.

use git::Git;
use git::read::changes::{SkippedRequest, parse_skipped, read_skipped};
use model::state::{Skipped, SkippedFile};
use testkit::Repository;

use super::read;

fn skipped(path: &str, how: Skipped) -> SkippedFile {
    SkippedFile {
        path: path.to_owned(),
        how,
    }
}

fn request(repository: &Repository) -> SkippedRequest {
    SkippedRequest {
        git_folder: repository.path().join(".git"),
        before: None,
    }
}

#[test]
fn changes_read_skipped__the_letters_of_ls_files_v_say_how_each_file_is_hidden() {
    let read = parse_skipped(b"h a.txt\0S b.txt\0s c.txt\0H d.txt\0", false);
    assert_eq!(
        read,
        [
            skipped("a.txt", Skipped::AssumedUnchanged),
            skipped("b.txt", Skipped::Worktree),
            skipped("c.txt", Skipped::Both),
        ]
    );
}

#[test]
fn changes_read_skipped__a_path_with_spaces_and_letters_beyond_ascii_comes_back_as_the_index_has_it() {
    let mut repository = Repository::new();
    let name = "notes été/my file.txt";
    let _first = repository.commit("Add the notes", &[(name, "notes\n")]);
    let _skipped = repository.git(&["update-index", "--skip-worktree", "--", name]);
    assert_eq!(read(&repository).skipped, [skipped(name, Skipped::Worktree)]);
}

#[test]
fn changes_read_skipped__in_a_sparse_checkout_a_file_marked_s_is_left_out_and_one_marked_h_is_kept() {
    assert_eq!(
        parse_skipped(b"S out.txt\0h kept.txt\0", true),
        [skipped("kept.txt", Skipped::AssumedUnchanged)]
    );
    let mut repository = Repository::new();
    let _first = repository.commit(
        "Add the folders",
        &[("in/a.txt", "a\n"), ("out/b.txt", "b\n"), ("kept.txt", "k\n")],
    );
    let _sparse = repository.git(&["sparse-checkout", "set", "in"]);
    let _assumed = repository.git(&["update-index", "--assume-unchanged", "--", "kept.txt"]);
    let state = read(&repository);
    assert!(
        !state.skipped.iter().any(|file| file.path == "out/b.txt"),
        "{:?}",
        state.skipped
    );
}

#[test]
fn changes_read_skipped__a_change_to_a_file_of_the_working_tree_only_reads_no_ls_files_again() {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a\n")]);
    let git = Git::new(repository.path());
    let mut first = read_skipped(&git, &request(&repository)).unwrap();
    assert_eq!(first.files, Vec::<SkippedFile>::new());
    // A file the index does not hide, which a read of git would not give: kept, it shows that git did not run.
    first
        .files
        .push(skipped("kept from the read before", Skipped::Worktree));
    std::fs::write(repository.path().join("a.txt"), "changed\n").unwrap();
    let again = SkippedRequest {
        before: Some(first.clone()),
        ..request(&repository)
    };
    assert_eq!(read_skipped(&git, &again).unwrap(), first);
    let _hidden = repository.git(&["update-index", "--assume-unchanged", "--", "a.txt"]);
    assert_eq!(
        read_skipped(&git, &again).unwrap().files,
        [skipped("a.txt", Skipped::AssumedUnchanged)]
    );
}
