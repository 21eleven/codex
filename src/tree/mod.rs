use crate::node::{power_of_ten, prepare_path_name, Node, NodeKey, NodeMeta};
use crate::utils::commit_paths;
use chrono::Local;
use git2::Repository;
use log::*;
use nom::bytes::complete::{tag, take_till};
use nom::IResult;
use nvim_rs::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::error;
use std::fmt;
use std::fs::rename;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

pub type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

#[derive(Debug)]
pub struct Tree {
    pub nodes: BTreeMap<NodeKey, Node>,
    pub journal: NodeKey,
    pub desk: NodeKey,
}

impl fmt::Display for Tree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Tree(\n")?;
        let keys: Vec<String> = self.nodes.keys().map(|p| p.to_owned()).collect();
        for id in keys {
            write!(f, "\t {}\n", id)?;
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

pub fn next_sibling_id(key: &String) -> u64 {
    let path = PathBuf::from(key);
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

pub fn next_child_id(path: &String) -> u64 {
    let path = PathBuf::from(path);
    // why not just look in child array of parent?
    // TODO: check search_dir exists?
    let search_dir = PathBuf::from("./codex/").join(path);
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

pub fn get_parent(key: &NodeKey) -> Option<NodeKey> {
    // should be result?
    match key.rsplit_once('/') {
        Some((parent, _)) => Some(parent.to_string()),
        None => None,
    }
}

fn get_node_key_number(node_key: &NodeKey) -> u64 {
    // should be a result?
    let (_base, node) = node_key.as_str().rsplit_once('/').unwrap();
    let (x, _name_path) = node.split_once('-').unwrap();
    x.parse::<u64>().unwrap()
}

impl Tree {
    pub fn build(root: String) -> Result<Tree> {
        fn dfs(
            name: Option<NodeKey>,
            node_map: &mut BTreeMap<NodeKey, Node>,
            parent: Option<NodeKey>,
            siblings: Vec<NodeKey>,
            journal: &mut Option<NodeKey>,
            desk: &mut Option<NodeKey>,
            base: &Path,
        ) {
            let n = base.to_str().unwrap().chars().count();
            let search_dir = match name.clone() {
                None => base.to_path_buf(),
                Some(name_path) => base.join(PathBuf::from(name_path)),
            };
            let children = WalkDir::new(search_dir)
                .sort_by_file_name()
                .contents_first(true)
                .min_depth(2)
                .max_depth(2)
                .into_iter()
                .map(|entry| entry.unwrap())
                .filter(|path| path.file_name().to_str().unwrap().ends_with("meta.toml"))
                .map(|e| {
                    e.into_path()
                        .parent()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .chars()
                        .skip(n)
                        .collect::<String>()
                })
                .collect::<Vec<NodeKey>>();
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
        let mut node_map: BTreeMap<NodeKey, Node> = BTreeMap::new();
        let mut journal: Option<NodeKey> = None;
        let mut desk: Option<NodeKey> = None;
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
    pub fn today_node(&mut self) -> NodeKey {
        // TODO: some way for user to pass in strf string
        let today = Local::now().format("%a %b %d %Y");
        let journal = self.journal.clone();
        let journal_node = self.nodes.get(&self.journal).unwrap();
        let newest_child = journal_node.children[journal_node.children.len() - 1].clone();
        debug!(
            "newest child {} today {}",
            newest_child,
            prepare_path_name(&today.to_string())
        );
        if newest_child.ends_with(&prepare_path_name(&today.to_string())) {
            newest_child
        } else {
            debug!("creating new day node for {}", &today);
            self.create_node(Some(&journal), Some(&today.to_string()))
                .unwrap()
        }
    }
    pub fn node_creation(&mut self, args: Vec<Value>) {
        let args: Vec<Option<&str>> = args.iter().map(|arg| arg.as_str()).collect();
        match args.as_slice() {
            &[Some(parent), Some(child)] => {
                self.create_node(Some(parent), Some(child)).unwrap();
            }
            &[Some(node_name)] => {
                self.create_node(None, Some(node_name)).unwrap();
            }
            _ => {
                error!("invalid args to create: {:?}", args);
            }
        }
    }
    pub fn create_node(&mut self, parent: Option<&str>, child: Option<&str>) -> Result<NodeKey> {
        // TODO: what would happen if the input had a '/'? sanatize
        // need to decouple this node creation on tree
        // from processing of RPC message pack value
        match (parent, child) {
            (Some(parent), Some(child)) => {
                let parent = parent.to_string();
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
                    let parent_ref = get_parent(&child_id).unwrap();
                    let mut siblings = self.nodes.get_mut(&parent_ref).unwrap().children.clone();
                    match power_of_ten(get_node_key_number(&child_id)) {
                        Some(n) => {
                            // TODO make this section more DRY
                            let repo = Repository::open("./").unwrap();
                            for idx in 0..siblings.len() {
                                if &siblings[idx] == &child_id {
                                    continue;
                                }
                                let sibid = &siblings[idx].clone();
                                // TODO make this a function -> (u64, &str)
                                let (base, node) = sibid.rsplit_once('/').unwrap();
                                let (x, name_path) = node.split_once('-').unwrap();
                                let x = x.parse::<u64>().unwrap();
                                let newid = format!(
                                    "{}/{:0width$}-{}",
                                    base,
                                    x,
                                    name_path,
                                    width = (n as usize) + 1
                                );
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
                                    node_ref: &NodeKey,
                                    parent: &NodeKey,
                                    map: &mut BTreeMap<NodeKey, Node>,
                                ) -> NodeKey {
                                    let mut node = map.remove(node_ref).unwrap();
                                    let (_, node_name) = node_ref.rsplit_once('/').unwrap();
                                    let newid = format!("{}/{}", parent, node_name,);
                                    for link in &node.links {
                                        let linked = map.get_mut(link).unwrap();
                                        linked.rename_backlink(&node_ref, &newid);
                                    }
                                    for backlink in &node.backlinks {
                                        let backlinked = map.get_mut(backlink).unwrap();
                                        backlinked.rename_link(&node_ref, &newid);
                                    }
                                    node.parent = Some(parent.to_string());
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
                                &format!("node renames due to new power of ten node {}", child_id),
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
                    Ok(child_id)
                } else {
                    error!("problem");
                    Err(Box::new(TreeError {
                        err_text: "child creation failed".to_string(),
                    }))
                }
            }
            (None, Some(node_name)) => {
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
                    .collect::<Vec<NodeKey>>();
                debug!("{:?}", siblings);
                let mut node = Node::create(node_name.to_string(), None);
                siblings.push(node.id.clone());
                node.siblings = siblings.clone();
                let node_id = node.id.clone();
                self.nodes.insert(node.id.clone(), node);
                debug!("{:?}", siblings);
                for node_ref in &siblings {
                    self.nodes.get_mut(node_ref).unwrap().siblings = siblings.clone();
                }
                Ok(node_id)
            }
            _ => {
                error!(
                    "invalid args to create_node: new node: {:?} parent: {:?}",
                    child, parent
                );
                Err(Box::new(TreeError {
                    err_text: format!(
                        "invalid args to create_node: new node: {:?} parent: {:?}",
                        child, parent
                    ),
                }))
            }
        }
    }
}
