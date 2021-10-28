use crate::node::{power_of_ten, Node, NodeMeta, NodeRef};
use crate::utils::commit_paths;
use git2::Repository;
use log::*;
use nom::bytes::complete::{tag, take_till};
use nom::IResult;
use nvim_rs::Value;
use std::collections::{HashMap, HashSet};
use std::error;
use std::fmt;
use std::fs::rename;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

pub type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

#[derive(Debug)]
pub struct Tree {
    pub nodes: HashMap<NodeRef, Node>,
    pub journal: NodeRef,
    pub desk: NodeRef,
}

impl fmt::Display for Tree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Tree(\n")?;
        let mut keys: Vec<PathBuf> = self.nodes.keys().map(|p| p.to_owned()).collect();
        keys.sort_unstable();
        for id in keys {
            write!(f, "\t {}\n", id.to_str().unwrap_or("error"))?;
        }
        write!(f, ")")?;
        Ok(())
    }
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

fn get_node_ref_number(node_ref: &NodeRef) -> u64 {
    let (_base, node) = node_ref
        .as_path()
        .to_str()
        .unwrap()
        .rsplit_once('/')
        .unwrap();
    let (x, _name_path) = node.split_once('-').unwrap();
    x.parse::<u64>().unwrap()
}

impl Tree {
    pub fn build(root: String) -> Result<Tree> {
        fn dfs(
            name: Option<PathBuf>,
            node_map: &mut HashMap<NodeRef, Node>,
            parent: Option<NodeRef>,
            siblings: Vec<NodeRef>,
            journal: &mut Option<NodeRef>,
            desk: &mut Option<NodeRef>,
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
                    journal,
                    desk,
                    base,
                );
            }
            match name {
                Some(namepath) => {
                    debug!("{:?}", &namepath);
                    let meta_path = base.join(&namepath).join("meta.toml");
                    let node = Node::from_tree(namepath, &meta_path, parent, siblings, children);
                    if journal.is_none() && node.tags.contains("journal") {
                        *journal = Some(node.id.clone());
                    }
                    if desk.is_none() && node.tags.contains("desk") {
                        *desk = Some(node.id.clone());
                    }
                    debug!("{}", &node);
                    node_map.insert(node.id.clone(), node);
                }
                None => {}
            }
        }
        let mut node_map: HashMap<NodeRef, Node> = HashMap::new();
        let mut journal: Option<NodeRef> = None;
        let mut desk: Option<NodeRef> = None;
        dfs(
            None,
            &mut node_map,
            None,
            vec![],
            &mut journal,
            &mut desk,
            Path::new(&root),
        );
        Ok(Tree {
            nodes: node_map,
            journal: journal.unwrap(),
            desk: desk.unwrap(),
        })
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
                    let mut siblings = self.nodes.get_mut(&parent_ref).unwrap().children.clone();
                    match power_of_ten(get_node_ref_number(&child_id)) {
                        Some(n) => {
                            // TODO make this section more DRY
                            let repo = Repository::open("./").unwrap();
                            for idx in 0..siblings.len() {
                                if &siblings[idx] == &child_id {
                                    continue;
                                }
                                let sibid = &siblings[idx].clone();
                                // TODO make this a function -> (u64, &str)
                                let (base, node) =
                                    sibid.as_path().to_str().unwrap().rsplit_once('/').unwrap();
                                let (x, name_path) = node.split_once('-').unwrap();
                                let x = x.parse::<u64>().unwrap();
                                let newid = PathBuf::from(format!(
                                    "{}/{:0width$}-{}",
                                    base,
                                    x,
                                    name_path,
                                    width = (n as usize) + 1
                                ));
                                siblings[idx] = newid.clone();
                                let mut node_clone = self.nodes.remove(sibid).unwrap();
                                // node_clone.mv(newid.clone());
                                node_clone.id = newid.clone();
                                debug!("renaming {:?} to {:?}", sibid, &newid);
                                let old_path = PathBuf::from("./codex").join(&sibid);
                                let new_path = PathBuf::from("./codex").join(&newid);
                                rename(old_path, new_path).unwrap();
                                // link is another node
                                // that this node points to in its content
                                for link in &node_clone.links {
                                    let linked = self.nodes.get_mut(link).unwrap();
                                    linked.rename_backlink(&sibid, &newid);
                                }
                                // a backlink is a node that has a link
                                // in its content that points to this node
                                for backlink in &node_clone.backlinks {
                                    let backlinked = self.nodes.get_mut(backlink).unwrap();
                                    backlinked.rename_link(&sibid, &newid);
                                }
                                // WHAT ABOUT THE CHILDREN???
                                fn rename_dfs(
                                    node_ref: &NodeRef,
                                    parent: &NodeRef,
                                    map: &mut HashMap<NodeRef, Node>,
                                ) -> NodeRef {
                                    let mut node = map.remove(node_ref).unwrap();
                                    let (_, node_name) = node_ref
                                        .as_path()
                                        .to_str()
                                        .unwrap()
                                        .rsplit_once('/')
                                        .unwrap();
                                    let newid = PathBuf::from(format!(
                                        "{}/{}",
                                        parent.to_str().unwrap(),
                                        node_name,
                                    ));
                                    for link in &node.links {
                                        let linked = map.get_mut(link).unwrap();
                                        linked.rename_backlink(&node_ref, &newid);
                                    }
                                    for backlink in &node.backlinks {
                                        let backlinked = map.get_mut(backlink).unwrap();
                                        backlinked.rename_link(&node_ref, &newid);
                                    }
                                    node.parent = Some(parent.to_path_buf());
                                    node.id = newid.clone();
                                    node.children = node
                                        .children
                                        .iter()
                                        .map(|child_ref| rename_dfs(child_ref, &newid, map))
                                        .collect();
                                    node.write_meta();
                                    map.insert(newid.clone(), node);
                                    newid
                                }
                                node_clone.children = node_clone
                                    .children
                                    .iter()
                                    .map(|child_ref| rename_dfs(child_ref, &newid, &mut self.nodes))
                                    .collect();
                                self.nodes.insert(newid, node_clone);
                            }
                            commit_paths(
                                &repo,
                                vec![&Path::new("codex/*")],
                                &format!(
                                    "node renames due to new power of ten node {}",
                                    &child_id.to_str().unwrap()
                                ),
                            )
                            .unwrap();
                        }
                        None => {}
                    }
                    for node_ref in &siblings {
                        debug!("resetting siblings array for {:?}", node_ref);
                        self.nodes.get_mut(node_ref).unwrap().siblings = siblings.clone();
                    }
                    self.nodes.get_mut(&parent_ref).unwrap().children = siblings;
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
