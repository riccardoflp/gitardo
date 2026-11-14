#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::{FilePath, TreeNode, file_tree};

/// The tree of some paths, drawn one row per line, indented by depth, directories ending with `/`.
fn drawn(paths: &[&str]) -> Vec<String> {
    let input: Vec<FilePath> = paths.iter().copied().map(FilePath::from).collect();
    let mut rows = Vec::new();
    draw(&file_tree(&input), 0, &mut rows);
    rows
}

fn draw(nodes: &[TreeNode], depth: usize, rows: &mut Vec<String>) {
    for node in nodes {
        let indent = "  ".repeat(depth);
        match node {
            TreeNode::Directory { name, children, .. } => {
                rows.push(format!("{indent}{name}/"));
                draw(children, depth + 1, rows);
            }
            TreeNode::File { name, .. } => rows.push(format!("{indent}{name}")),
        }
    }
}

#[test]
fn details_file_tree__directories_are_listed_before_files_at_each_level() {
    assert_eq!(
        drawn(&["a/b.txt", "a/c/d.txt", "z.txt", "b/x.txt"]),
        ["a/", "  c/", "    d.txt", "  b.txt", "b/", "  x.txt", "z.txt"]
    );
}

#[test]
fn details_file_tree__a_chain_of_directories_with_a_single_directory_in_each_is_one_row() {
    assert_eq!(
        drawn(&["src/git/ops.rs", "src/git/repo.rs", "README.md"]),
        ["src/git/", "  ops.rs", "  repo.rs", "README.md"]
    );
    assert_eq!(
        drawn(&["a/b/c/d/e.txt", "a/b/f.txt"]),
        ["a/b/", "  c/d/", "    e.txt", "  f.txt"]
    );
}

#[test]
fn details_file_tree__the_top_of_the_tree_is_never_joined_and_a_directory_with_a_file_stays_apart() {
    assert_eq!(drawn(&["only/one.txt"]), ["only/", "  one.txt"]);
    assert_eq!(drawn(&["a/b/x.txt", "a/y.txt"]), ["a/", "  b/", "    x.txt", "  y.txt"]);
}

#[test]
fn details_file_tree__every_input_path_appears_once_and_the_count_of_files_is_unchanged() {
    let paths = [
        "src/main.rs",
        "src/git/exec.rs",
        "src/git/read/repo.rs",
        "docs/a b.md",
        "Cargo.toml",
        "pages/[id].tsx",
        "accentué/é.txt",
    ];
    let input: Vec<FilePath> = paths.iter().copied().map(FilePath::from).collect();
    let tree = file_tree(&input);
    assert_eq!(tree.iter().map(TreeNode::file_count).sum::<usize>(), paths.len());
    let mut found = Vec::new();
    collect(&tree, &mut found);
    found.sort_unstable();
    assert_eq!(found, (0..paths.len()).collect::<Vec<_>>());
    for node in &tree {
        assert_eq!(node_paths_match(node, &input), Some(()));
    }
}

fn collect(nodes: &[TreeNode], found: &mut Vec<usize>) {
    for node in nodes {
        match node {
            TreeNode::Directory { children, .. } => collect(children, found),
            TreeNode::File { index, .. } => found.push(*index),
        }
    }
}

/// Whether each file of a node has the path of its place in the input.
fn node_paths_match(node: &TreeNode, input: &[FilePath]) -> Option<()> {
    match node {
        TreeNode::Directory { children, .. } => children.iter().try_for_each(|child| node_paths_match(child, input)),
        TreeNode::File { path, index, .. } => (input.get(*index)? == path).then_some(()),
    }
}

#[test]
fn details_file_tree__names_are_ordered_without_regard_to_case_then_by_their_characters() {
    assert_eq!(
        drawn(&["b.txt", "A.txt", "a.txt", "C/x", "b/y"]),
        ["b/", "  y", "C/", "  x", "A.txt", "a.txt", "b.txt"]
    );
}

#[test]
fn details_file_tree__the_path_of_a_joined_directory_names_its_whole_chain() {
    let tree = file_tree(&[FilePath::from("src/git/read/repo.rs"), FilePath::from("src/main.rs")]);
    let [TreeNode::Directory { children, path, .. }] = tree.as_slice() else {
        panic!("one directory at the top: {tree:?}")
    };
    assert_eq!(path, "src");
    let TreeNode::Directory { name, path, .. } = &children[0] else {
        panic!("a directory first")
    };
    assert_eq!((name.as_str(), path.as_str()), ("git/read", "src/git/read"));
}
