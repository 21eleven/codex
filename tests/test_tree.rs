#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_macros,
    unused_assignments,
    unused_mut
)]
use codex::node::{init_codex_repo, NodeLink};
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
    assert_eq!(
        nodekeys_in_dir(tempdir.path()),
        vec!["1-journal".to_string(), "2-desk".to_string()]
    );
}

#[rstest]
fn build_tree(dir_and_tree: (TempDir, Tree)) {
    let (dir, tree) = dir_and_tree;
    assert_eq!(number_of_nodes(dir.path()), 2);
    assert_eq!(tree.nodes.keys().count(), 2);
}

#[rstest]
fn create_and_link_nodes(dir_and_tree: (TempDir, Tree)) {
    let dir = dir_and_tree.0;
    let mut tree = dir_and_tree.1;
    assert_eq!(number_of_nodes(dir.path()), 2);
    let a = tree.create_node(Some("2-desk"), Some("a")).unwrap();
    let b = tree.create_node(Some(&a), Some("b")).unwrap();
    let c = tree.create_node(Some("2-desk"), Some("c")).unwrap();
    assert_eq!(number_of_nodes(dir.path()), 5);
    assert!(tree.nodes.contains_key("2-desk/1-a/1-b"));
    let link_id = "link".to_string();
    tree.link(link_id.clone(), &b, 0, 0, &c, 0, 0);
    let bnode = tree.nodes.get(&b).unwrap();
    assert!(bnode.links.contains_key(&link_id));
    let link = bnode.links.get(&link_id).unwrap().clone();
    let cnode = tree.nodes.get(&c).unwrap();
    assert!(cnode
        .backlinks
        .contains_key(&(link_id.clone(), link.timestamp)));
    let backlink = cnode
        .backlinks
        .get(&(link_id.clone(), link.timestamp))
        .unwrap()
        .clone();
    assert!(meta_has_link(bnode.metadata_path(), &link_id, &link));
    assert!(meta_has_backlink(
        cnode.metadata_path(),
        &link_id,
        &backlink
    ));
    let cnode_meta_toml = std::fs::read_to_string(cnode.metadata_path())
        .unwrap()
        .to_string();
    assert!(
        cnode_meta_toml.contains(&backlink.to_toml(&NodeLink::serialize_backlink_id(
            &link_id,
            backlink.timestamp
        )))
    );
}
