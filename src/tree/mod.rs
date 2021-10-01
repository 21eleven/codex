use crate::node::{Node, NodeRef, NodeMeta};
use log::*;
use std::collections::{HashMap, HashSet};
use std::error;
use std::fmt;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};
use std::fs::read_to_string;

pub type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

#[derive(Debug)]
pub struct Tree {
    nodes: HashMap<NodeRef, Node>,
}

pub struct TreeError {
    err_text: String,
}

impl fmt::Display for TreeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.err_text)
    }
}

impl fmt::Debug for TreeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TreeError( {} )", self.err_text)
    }
}

impl error::Error for TreeError {}

pub struct NodeFilesMissingError {
    content_file_exists: bool,
    metadata_file_exists: bool,
    node: String,
}

impl NodeFilesMissingError {
    fn err_text(&self) -> String {
        format!(
            "{} {}",
            match (self.content_file_exists, self.metadata_file_exists) {
                (false, true) => "Missing `_.md` for ",
                (true, false) => "Missing `meta.toml` for ",
                _ => "Missing `_.md` and `meta.toml` for ",
            }
            .to_string(),
            self.node
        )
    }
}

impl fmt::Display for NodeFilesMissingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.err_text())
    }
}

impl fmt::Debug for NodeFilesMissingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "NodeFilesMissingError( {} )", self.err_text())
    }
}

impl error::Error for NodeFilesMissingError {}

fn is_metadata_toml(entry: &DirEntry) -> bool {
    debug!("{:?}", entry.clone());
    entry
        .clone()
        .file_name()
        .to_str()
        .unwrap()
        .ends_with("meta.toml")
    // .map(|s| s.ends_with("meta.toml"))
    //.unwrap()//_or(false)
}

pub fn new_sibling_id(path: &PathBuf) -> u64 {
    let search_dir = match path.parent() {
        Some(parent) => PathBuf::from("./codex/").join(parent),
        None => PathBuf::from("./codex/"),
    };
    WalkDir::new(search_dir)
        .sort_by_file_name()
        .contents_first(true)
        .min_depth(1)
        .max_depth(2)
        .into_iter()
        .filter_entry(|e| is_metadata_toml(e))
        .map(|e| e.unwrap().into_path())
        .collect::<Vec<PathBuf>>()
        .len() as u64
}

impl Tree {
    pub fn build(root: String) -> Result<Tree> {
        fn dfs(
            name: Option<PathBuf>,
            node_map: &mut HashMap<NodeRef, Node>,
            parent: Option<NodeRef>,
            siblings: Vec<NodeRef>,
            base: &Path,
        ) {
            // start w root dir
            // find whats in cwd
            // verify _.md and meta.toml
            // ignore other non-dirs
            let n = base.to_str().unwrap().chars().count();
            let search_dir = match name.clone() {
                None => base.to_path_buf(),
                Some(name_path) => base.join(name_path.as_path()),
            };
            let children = WalkDir::new(search_dir)
                .sort_by_file_name()
                .contents_first(true)
                .min_depth(2)
                .max_depth(2)
                .into_iter()
                .map(|e| e.unwrap())
                .filter(|path| path.file_name().to_str().unwrap().ends_with("meta.toml"))
                .map(|e| {
                    PathBuf::from(
                        e.into_path()
                            .parent()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .chars()
                            .skip(n)
                            .collect::<String>(),
                    )
                })
                .collect::<Vec<PathBuf>>();
            for node in &children {
                dfs(
                    Some(node.clone()),
                    node_map,
                    name.clone(),
                    children.clone(),
                    base,
                );
            }
            match name {
                Some(namepath) => {
                    debug!("{:?}", &namepath);
                    let meta_path = base.join(&namepath).join("meta.toml");
                    let node = Node::from_tree(namepath, &meta_path, parent, siblings, children);
                    debug!("{:?}", &node);
                    node_map.insert(node.id.clone(), node);
                }
                None => {}
            }
        }
        let mut file_check: HashSet<PathBuf> = HashSet::new();
        let mut node_map: HashMap<NodeRef, Node> = HashMap::new();
        dfs(None, &mut node_map, None, vec![], Path::new(&root));
        debug!("{:?}", node_map);
        // for fs_node in WalkDir::new(root.as_str())
        //     .sort_by_file_name()
        //     .contents_first(true)
        //     .min_depth(1)
        // //skips root dir
        // {
        //     debug!("{:?}", fs_node);
        //     match fs_node {
        //         Ok(node_path) => {
        //             if !node_path.path().is_dir() {
        //                 // should *always* encounter node files fites
        //                 // when dir is encounter will check in set to
        //                 // verify dir struct not corrupt
        //                 file_check.insert(node_path.into_path());
        //             } else {
        //                 match (
        //                     file_check.contains(&node_path.path().join("_.md")),
        //                     file_check.contains(&node_path.path().join("meta.toml")),
        //                 ) {
        //                     (true, true) => {
        //                         // build node
        //                         // todo!()
        //                     }
        //                     (c1, c2) => {
        //                         return Err(NodeFilesMissingError {
        //                             content_file_exists: c1,
        //                             metadata_file_exists: c2,
        //                             node: {
        //                                 match node_path.path().to_str() {
        //                                     Some(path) => String::from(path),
        //                                     _ => "".to_owned(),
        //                                 }
        //                             },
        //                         }
        //                         .into())
        //                     }
        //                 }
        //             }
        //         }
        //         Err(e) => return Err(Box::new(e)),
        //     }
        // }
        Ok(Tree { nodes: node_map })
    }
}
// pub fn discover_tree(root: String) -> Result<Tree> {
//     Ok(Tree { chk: true })
// }
