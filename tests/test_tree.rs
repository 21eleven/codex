#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_macros,
    unused_assignments,
    unused_mut,
)]
use codex::node::init_codex_repo;
use codex::tree::Tree;

use rstest::rstest;
use rstest::*;
mod fixtures;
use fixtures::*;
mod utils;
use utils::*;




#[rstest]
fn blank_slate(tempdir: TempDir) {
    assert_eq!(number_of_nodes(tempdir.path()), 0);
    dbg!(&tempdir);
    init_codex_repo(Some(tempdir.path().to_str().unwrap()));
    assert_eq!(number_of_nodes(tempdir.path()), 2);
}

#[rstest]
fn build_tree(initialdir: TempDir) {
    assert_eq!(number_of_nodes(initialdir.path()), 2);
    dbg!(&initialdir);
    let tree = Tree::build(initialdir.path().to_str().unwrap()).unwrap();
    assert_eq!(tree.nodes.keys().count(), 2);

}
// fn link_nodes(tree_ab: TreeAB) {
//     // tree_ab is a fixture struct and that hold as tree and the ids for two nodes within it
//
// }
