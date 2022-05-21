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

#[rstest]
fn sibling_count_order_of_magnitude_rollover(dir_and_tree: (TempDir, Tree)) {
    let dir = dir_and_tree.0;
    let mut tree = dir_and_tree.1;
    assert_eq!(number_of_nodes(dir.path()), 2);
    let one = tree.create_node(Some("2-desk"), Some("one")).unwrap();
    let child = tree.create_node(Some(&one), Some("child")).unwrap();
    let two = tree.create_node(Some("2-desk"), Some("two")).unwrap();
    assert_eq!(number_of_nodes(dir.path()), 5);
    assert!(tree.nodes.contains_key("2-desk/1-one/1-child"));
    let link_id = "link".to_string();
    tree.link(&link_id, &child, 0, 0, &two, 0, 0);
    tree.link("one", &two, 100, 10, &one, 0, 0);

    // ten total nodes in desk child group, order of magnitude rollover
    tree.create_node(Some("2-desk"), Some("three")).unwrap();
    tree.create_node(Some("2-desk"), Some("four")).unwrap();
    tree.create_node(Some("2-desk"), Some("five")).unwrap();
    tree.create_node(Some("2-desk"), Some("six")).unwrap();
    tree.create_node(Some("2-desk"), Some("seven")).unwrap();
    tree.create_node(Some("2-desk"), Some("eight")).unwrap();
    tree.create_node(Some("2-desk"), Some("nine")).unwrap();
    tree.create_node(Some("2-desk"), Some("ten")).unwrap();

    let one = "2-desk/01-one";
    let two = "2-desk/02-two";
    let child = "2-desk/01-one/1-child";
    assert!(tree.nodes.contains_key(child));
    assert!(tree.nodes.contains_key("2-desk/10-ten"));

    let onenode = tree.nodes.get(one).unwrap();
    let childnode = tree.nodes.get(child).unwrap();
    let twonode = tree.nodes.get(two).unwrap();

    // TODO this could be its own function since we do the 
    // same link integrity check in another test
    let two_to_one_backlink = onenode.backlinks.values().take(1).next().unwrap();
    assert!(&two_to_one_backlink.node == two);
    let link = childnode.links.get(&link_id).unwrap().clone();
    assert!(&link.node == two);
    assert!(twonode
        .backlinks
        .contains_key(&(link_id.clone(), link.timestamp)));
    let backlink = twonode
        .backlinks
        .get(&(link_id.clone(), link.timestamp))
        .unwrap()
        .clone();
    assert!(meta_has_link(childnode.metadata_path(), &link_id, &link));
    assert!(meta_has_backlink(
        twonode.metadata_path(),
        &link_id,
        &backlink
    ));
    let cnode_meta_toml = std::fs::read_to_string(twonode.metadata_path())
        .unwrap()
        .to_string();
    dbg!(&cnode_meta_toml);
    dbg!(&backlink);
    dbg!(&backlink.to_toml());
    dbg!(&two_to_one_backlink);
    dbg!(&two_to_one_backlink.to_toml());
    assert!(cnode_meta_toml.contains(&backlink.to_toml()));
}
