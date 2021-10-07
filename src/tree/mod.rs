use crate::node::{Node, NodeMeta, NodeRef};
use log::*;
use nom::bytes::complete::{tag, take_till};
use nom::IResult;
use nvim_rs::Value;
use std::collections::{HashMap, HashSet};
use std::error;
use std::fmt;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

pub type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

#[derive(Debug)]
pub struct Tree {
    pub nodes: HashMap<NodeRef, Node>,
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

pub fn next_sibling_id(path: &PathBuf) -> u64 {
    // could be next root dir id
    let search_dir = match path.parent() {
        Some(parent) => PathBuf::from("./codex/").join(parent),
        None => PathBuf::from("./codex/"),
    };
    // TODO: check search_dir exists?
    let metas = WalkDir::new(search_dir)
        .sort_by_file_name()
        .contents_first(true)
        .min_depth(2)
        .max_depth(2)
        .into_iter()
        .map(|e| e.unwrap().into_path())
        .filter(|p| p.is_file() && p.ends_with("meta.toml"))
        .collect::<Vec<PathBuf>>();
    metas.len() as u64 + 1
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
        let mut node_map: HashMap<NodeRef, Node> = HashMap::new();
        dfs(None, &mut node_map, None, vec![], Path::new(&root));
        debug!("{:?}", node_map);
        Ok(Tree { nodes: node_map })
    }
    pub fn create_node(&mut self, args: Vec<Value>) {
        // TODO: what would happen if the input had a '/'? sanatize
        let args: Vec<Option<&str>> = args.iter().map(|arg| arg.as_str()).collect();
        match args.as_slice() {
            &[Some(parent), Some(child)] => {
                let parent = PathBuf::from(parent);
                debug!("parent {:?} and child {:?}", parent, child);
                let child = match self.nodes.get_mut(&parent) {
                    Some(parent) => Some(parent.create_child(child.to_string())),
                    None => {
                        error!("no node in tree named: {:?}", parent);
                        None
                    }
                };
                if let Some(child) = child {
                    let child_id = child.id.clone();
                    self.nodes.insert(child.id.clone(), child);
                    let parent_ref = child_id.parent().unwrap().to_path_buf();
                    let siblings = self.nodes.get_mut(&parent_ref).unwrap().children.clone();
                    for node_ref in &siblings {
                        self.nodes.get_mut(node_ref).unwrap().siblings = siblings.clone();
                    }
                }
            }
            &[Some(node_name)] => {
                // TODO what is the right way to remove this hard coding?
                // 'static or const?
                let root = PathBuf::from("./codex");
                fn parse_name(input: &str) -> IResult<&str, &str> {
                    let (input, _) = tag("./codex/")(input)?;
                    take_till(|c| c == '/')(input)
                }
                let mut siblings = WalkDir::new(root)
                    .sort_by_file_name()
                    .contents_first(true)
                    .min_depth(2)
                    .max_depth(2)
                    .into_iter()
                    .map(|e| e.unwrap().into_path())
                    .filter(|p| p.is_file() && p.ends_with("meta.toml"))
                    .map(|p| {
                        parse_name(p.as_path().to_str().unwrap())
                            .unwrap()
                            .1
                            .to_string()
                    })
                    .map(|s| PathBuf::from(s))
                    .collect::<Vec<PathBuf>>();
                debug!("{:?}", siblings);
                let mut node = Node::create(node_name.to_string(), None);
                siblings.push(node.id.clone());
                node.siblings = siblings.clone();
                self.nodes.insert(node.id.clone(), node);
                debug!("{:?}", siblings);
                for node_ref in &siblings {
                    self.nodes.get_mut(node_ref).unwrap().siblings = siblings.clone();
                }
            }
            _ => {
                error!("invalid args to create: {:?}", args);
            }
        }
    }
}
