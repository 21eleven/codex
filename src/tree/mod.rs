use crate::git::{commit_paths, stage_all};
use crate::node::{power_of_ten, prepare_path_name, Node, NodeKey, NodeMeta};
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

impl Drop for Tree {
    fn drop(&mut self) {
        debug!("dropping codex node tree");
    }
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
        Some(parent) => PathBuf::from("./").join(parent),
        None => PathBuf::from("./"),
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
    let search_dir = PathBuf::from("./").join(path);
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
            for node in children.iter() {
                dfs(
                    Some(node.clone()),
                    node_map,
                    name.clone(),
                    journal,
                    desk,
                    base,
                );
            }
            match name {
                Some(namepath) => {
                    // debug!("{:?}", &namepath);
                    let meta_path = base.join(&namepath).join("meta.toml");
                    let node = Node::from_tree(namepath, &meta_path, parent, children);
                    if journal.is_none() && node.tags.contains("journal") {
                        *journal = Some(node.id.clone());
                    }
                    if desk.is_none() && node.tags.contains("desk") {
                        *desk = Some(node.id.clone());
                    }
                    // debug!("{}", &node);
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
        if journal_node.children.is_empty() {
            debug!("creating first day node: {}", &today);
            return self
                .create_node(Some(&journal), Some(&today.to_string()))
                .unwrap();
        }
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
    pub fn node_creation(&mut self, args: Vec<Value>) -> Result<NodeKey> {
        let args: Vec<Option<&str>> = args.iter().map(|arg| arg.as_str()).collect();
        match args.as_slice() {
            [Some(parent), Some(child)] => Ok(self.create_node(Some(parent), Some(child)).unwrap()),
            [Some(node_name)] => Ok(self.create_node(None, Some(node_name)).unwrap()),
            _ => {
                error!("invalid args to create: {:?}", args);
                Err(Box::new(TreeError {
                    err_text: format!("invalid args to node_creation: {:?}", args),
                }))
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
                    // a new node is created, it has a parent
                    let child_id = child.id.clone();
                    self.nodes.insert(child.id.clone(), child);
                    stage_all().unwrap();
                    let parent_ref = get_parent(&child_id).unwrap();
                    if let Some(n) = power_of_ten(get_node_key_number(&child_id)) {
                        // this newly created node is a power of 10 node
                        // we must go to all the siblings and rename them
                        // TODO make this section more DRY
                        let repo = Repository::open("./").unwrap();
                        let siblings = self.nodes.get_mut(&parent_ref).unwrap().children.clone();
                        for idx in 0..siblings.len() {
                            if &siblings[idx] == &child_id {
                                // the new nodes is named in the
                                // siblings vec, it does not need
                                // rename
                                continue;
                            }
                            // get current name of siblings
                            let sibid = &siblings[idx].clone();
                            // TODO make this a function -> (u64, &str)
                            let (base, node) = sibid.rsplit_once('/').unwrap();
                            let (x, name_path) = node.split_once('-').unwrap();
                            let x = x.parse::<u64>().unwrap();
                            // new name of sibling
                            let newid = format!(
                                "{}/{:0width$}-{}",
                                base,
                                x,
                                name_path,
                                width = (n as usize) + 1
                            );
                            // remove sib node w old key from map
                            let mut node_clone = self.nodes.remove(sibid).unwrap();
                            // node_clone.mv(newid.clone());
                            node_clone.id = newid.clone();
                            debug!("renaming {:?} to {:?}", sibid, &newid);
                            let old_path = PathBuf::from(".").join(&sibid);
                            let new_path = PathBuf::from(".").join(&newid);
                            // move sib node on fs
                            rename(old_path, new_path).unwrap();
                            // link is another node
                            // that this node points to in its content
                            for link in &node_clone.links {
                                let linked = self.nodes.get_mut(link).unwrap();
                                linked.rename_backlink(sibid, &newid);
                            }
                            // a backlink is a node that has a link
                            // in its content that points to this node
                            for backlink in &node_clone.backlinks {
                                let backlinked = self.nodes.get_mut(backlink).unwrap();
                                backlinked.rename_link(sibid, &newid);
                            }
                            // all children need to be renamed since their
                            // parent has a new id and that suffixes their
                            // id
                            // could refactor this to have the renameing based on
                            // width happen in here
                            fn rename_dfs(
                                node_ref: &NodeKey,
                                parent: &NodeKey,
                                map: &mut BTreeMap<NodeKey, Node>,
                            ) -> NodeKey {
                                // remove child from amp
                                let mut node = map.remove(node_ref).unwrap();
                                let (_, node_name) = node_ref.rsplit_once('/').unwrap();
                                // calc new name
                                let newid = format!("{}/{}", parent, node_name,);
                                // inform links
                                for link in &node.links {
                                    let linked = map.get_mut(link).unwrap();
                                    linked.rename_backlink(node_ref, &newid);
                                }
                                for backlink in &node.backlinks {
                                    let backlinked = map.get_mut(backlink).unwrap();
                                    backlinked.rename_link(node_ref, &newid);
                                }
                                node.parent = Some(parent.to_string());
                                node.id = newid.clone();
                                for i in 0..node.children.len() {
                                    node.children[i] = rename_dfs(&node.children[i], &newid, map)
                                }
                                node.write_meta();
                                map.insert(newid.clone(), node);
                                newid
                            }
                            for i in 0..node_clone.children.len() {
                                node_clone.children[i] =
                                    rename_dfs(&node_clone.children[i], &newid, &mut self.nodes)
                            }
                            self.nodes.insert(newid, node_clone);
                        }
                        commit_paths(
                            &repo,
                            vec![Path::new("./*")],
                            &format!("node renames due to new power of ten node {}", child_id),
                        )
                        .unwrap();
                    }
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
                // TODO how to handle power of 10 rename here?
                let node = Node::create(node_name.to_string(), None);
                let node_id = node.id.clone();
                self.nodes.insert(node_id.clone(), node);
                stage_all().unwrap();
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
