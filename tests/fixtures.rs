use tempfile;

use codex::node::init_codex_repo;
use codex::tree::Tree;
use rstest::fixture;

pub type TempDir = tempfile::TempDir;

#[fixture]
pub fn tempdir() -> TempDir {
    TempDir::new().unwrap()
}

#[fixture]
pub fn initialdir(tempdir: TempDir) ->TempDir {
    init_codex_repo(Some(tempdir.path().to_str().unwrap()));
    tempdir
}

#[fixture]
pub fn dir_and_tree(initialdir: TempDir) ->(TempDir, Tree) {
    let tree = Tree::build(initialdir.path().to_str().unwrap()).unwrap();
    (initialdir, tree)
}

pub struct DirTreeNodes {
    dir: TempDir,
    tree: Tree,
    nodes: Vec<String>,
}

#[fixture]
pub fn dir_tree_nodes(dir_and_tree: (TempDir, Tree)) ->DirTreeNodes {
    let dir = dir_and_tree.0;
    let mut tree = dir_and_tree.1;
    let nodes = ["a", "b"].into_iter().map(|s| s.to_string()).collect::<Vec<String>>();
    for node in &nodes {
        tree.create_node(Some("2-desk"), Some(&node)).unwrap();
    }
    DirTreeNodes { dir, tree, nodes }

}
