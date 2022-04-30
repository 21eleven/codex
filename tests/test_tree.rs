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
    tree.link(&link_id, &b, 0, 0, &c, 0, 0);
    tree.link("a", &c, 100, 10, &a, 0, 0);
    let anode = tree.nodes.get(&a).unwrap();
    let bnode = tree.nodes.get(&b).unwrap();
    let cnode = tree.nodes.get(&c).unwrap();
    let c_to_a_backlink = anode.backlinks.values().take(1).next().unwrap();
    assert!(c_to_a_backlink.is_name_linked);
    assert!(c_to_a_backlink.to_toml().contains("name_ref"));
    assert!(&c_to_a_backlink.node == &c);
    assert!(bnode.links.contains_key(&link_id));
    let link = bnode.links.get(&link_id).unwrap().clone();
    assert!(&link.node == &c);
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
    dbg!(&cnode_meta_toml);
    dbg!(&backlink);
    dbg!(&backlink.to_toml());
    dbg!(&c_to_a_backlink);
    dbg!(&c_to_a_backlink.to_toml());
    assert!(cnode_meta_toml.contains(&backlink.to_toml()));
}

#[rstest]
fn node_index_parse(dir_and_tree: (TempDir, Tree)) {
    let dir = dir_and_tree.0;
    let mut tree = dir_and_tree.1;
    let a = tree.create_node(Some("2-desk"), Some("a")).unwrap();
    let b = tree.create_node(Some(&a), Some("b")).unwrap();
    let desk = tree.nodes.get(&"2-desk".to_string()).unwrap();
    assert_eq!(desk.index(), 2);
    let bnode = tree.nodes.get(&b).unwrap();
    assert_eq!(bnode.index(), 1);
}

#[rstest]
fn next_sibling(dir_and_tree: (TempDir, Tree)) {
    let dir = dir_and_tree.0;
    let mut tree = dir_and_tree.1;
    let a = tree.create_node(Some("2-desk"), Some("a")).unwrap();
    let b = tree.create_node(Some(&a), Some("b")).unwrap();
    let c = tree.create_node(Some("2-desk"), Some("c")).unwrap();
    let d = tree.create_node(Some("2-desk"), Some("d")).unwrap();
    assert_eq!(tree.next_sibling(&b, true), b);
    assert_eq!(tree.next_sibling(&b, false), b);
    assert_eq!(tree.next_sibling(&a, true), d);
    assert_eq!(tree.next_sibling(&a, false), c);
    assert_eq!(tree.next_sibling(&c, true), a);
    assert_eq!(tree.next_sibling(&c, false), d);
    assert_eq!(tree.next_sibling(&d, true), c);
    assert_eq!(tree.next_sibling(&d, false), a);
}
