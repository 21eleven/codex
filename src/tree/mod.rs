use crate::node::{power_of_ten, prepare_path_name, Node, NodeMeta, NodeRef};
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
pub struct Tree<'b> {
    pub nodes: BTreeMap<NodeRef<'b>, Node<'b>>,
    pub journal: &'b str,
    pub desk: NodeRef<'b>,
}

impl<'a> fmt::Display for Tree<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Tree(\n");
        let mut keys: Vec<&'a str> = self.nodes.keys().map(|p| p.to_owned()).collect();
        keys.sort_unstable();
        for id in keys {
            write!(f, "\t {}\n", id);
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

pub fn next_child_id(path: &PathBuf) -> u64 {
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

fn get_node_ref_number(node_ref: &NodeRef) -> u64 {
    let (_base, node) = node_ref
        .rsplit_once('/')
        .unwrap();
    let (x, _name_path) = node.split_once('-').unwrap();
    x.parse::<u64>().unwrap()
}

impl<'a> Tree<'a> {
    pub fn build(root: String) -> Result<Tree<'a>> {
        fn dfs<'a>(
            dirname: Option<&'a Path>,
            node_map: &mut BTreeMap<&'a str, Node<'a>>,
            parent: Option<&'a str>,
            siblings: Vec<&'a str>,
            journal: &mut Option<&'a str>,
            desk: &mut Option<&'a str>,
            base: &Path,
        ) {
            let n = base.to_str().unwrap().chars().count();
            let search_dir = match dirname.clone() {
                None => base.to_path_buf(),
                Some(name_path) => base.join(name_path),
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
                        e.into_path()
                            .parent()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .chars()
                            .skip(n)
                            .collect::<String>().as_str()
                })
                .collect::<Vec<&str>>();
            for node_dir in children {
                dfs(
                    Some(&Path::new(node_dir)),
                    node_map,
                    dirname.map(|path| path.to_str().unwrap()),
                    children.clone(),
                    journal,
                    desk,
                    base,
                );
            }
            match dirname {
                Some(namepath) => {
                    debug!("{:?}", namepath);
                    let meta_path = base.join(&namepath).join("meta.toml");
                    let node = Node::from_tree(namepath.to_str().unwrap(), &meta_path, parent, siblings, children);
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
        let mut node_map: BTreeMap<NodeRef, Node> = BTreeMap::new();
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
    pub fn today_node(&'a mut self) -> &'a str {
        // TODO: some way for user to pass in strf string
        let today = Local::now().format("%a %b %d %Y");
        let journal_node = self.nodes.get(&self.journal).unwrap();
        let newest_child = journal_node.children[journal_node.children.len() - 1];
        debug!(
            "newest child {} today {}",
            newest_child,
            prepare_path_name(&today.to_string())
        );
        if newest_child
            .ends_with(&prepare_path_name(&today.to_string()))
        {
            newest_child
        } else {
            debug!("creating new day node for {}", &today);
            self.create_node(Some(self.journal), Some(&today.to_string()))
                .unwrap()
        }
    }
    pub fn node_creation(&'a mut self, args: Vec<Value>) {
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
    pub fn create_node(&'a mut self, parent: Option<&'a str>, child: Option<&'a str>) -> Result<&'a str> {
        // TODO: what would happen if the input had a '/'? sanatize
        // need to decouple this node creation on tree
        // from processing of RPC message pack value
        match (parent, child) {
            (Some(parent), Some(child)) => {
                // let parent = PathBuf::from(parent);
                debug!("parent {:?} and child {:?}", parent, child);
                let child = match self.nodes.get_mut(parent) {
                    Some(parent) => Some(parent.create_child(child)),
                    None => {
                        error!("no node in tree named: {:?}", parent);
                        None
                    }
                };
                if let Some(child) = child {
                    let child_id = child.id.clone();
                    self.nodes.insert(child.id.clone(), child);
                    // let parent_ref = child_id.parent();
                    let mut siblings = self.nodes.get_mut(parent).unwrap().children.clone();
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
                                    sibid.rsplit_once('/').unwrap();
                                let (x, name_path) = node.split_once('-').unwrap();
                                let x = x.parse::<u64>().unwrap();
                                let newid = format!(
                                    "{}/{:0width$}-{}",
                                    base,
                                    x,
                                    name_path,
                                    width = (n as usize) + 1
                                );
                                siblings[idx] = newid.as_str();
                                let mut node_clone = self.nodes.remove(sibid).unwrap();
                                // node_clone.mv(newid.clone());
                                node_clone.id = newid.as_str();
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
                                fn rename_dfs<'a>(
                                    node_ref: &'a str,
                                    parent: &'a str,
                                    map: &'a mut BTreeMap<&'a str, Node<'a>>,
                                ) -> &'a str {
                                    let mut node = map.remove(node_ref).unwrap();
                                    let (_, node_name) = node_ref
                                        .rsplit_once('/')
                                        .unwrap();
                                    let newid = format!(
                                        "{}/{}",
                                        parent,
                                        node_name,
                                    );
                                    for link in &node.links {
                                        let linked = map.get_mut(link).unwrap();
                                        linked.rename_backlink(&node_ref, &newid);
                                    }
                                    for backlink in &node.backlinks {
                                        let backlinked = map.get_mut(backlink).unwrap();
                                        backlinked.rename_link(&node_ref, &newid);
                                    }
                                    node.parent = Some(parent);
                                    node.id = newid.as_str();
                                    // node.children = node
                                    //     .children
                                    //     .iter()
                                    //     .map(|child_ref| rename_dfs(child_ref, newid.as_str(), map))
                                    //     .collect();
                                    let mut children = vec![];
                                    for child in node.children {
                                        children.push(rename_dfs(child, newid.as_str(), map))
                                    }
                                    node.children = children;
                                    // must do this ^ instead... weird..
                                    node.write_meta();
                                    map.insert(newid.as_str(), node);
                                    newid.as_str()
                                }
                                // node_clone.children = node_clone
                                //     .children
                                //     .iter()
                                //     .map(|child_ref| rename_dfs(child_ref, &newid, &mut self.nodes))
                                //     .collect();
                                let mut children = vec![];
                                for child in node_clone.children {
                                    children.push(rename_dfs(child, newid.as_str(), &mut self.nodes))
                                }
                                node_clone.children = children;
                                self.nodes.insert(newid.as_str(), node_clone);
                            }
                            commit_paths(
                                &repo,
                                vec![&Path::new("codex/*")],
                                &format!(
                                    "node renames due to new power of ten node {}",
                                    &child_id
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
                    self.nodes.get_mut(parent).unwrap().children = siblings;
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
                    .map(|s| s.as_str())
                    .collect::<Vec<&'a str>>();
                debug!("{:?}", siblings);
                let mut node = Node::create(node_name, None);
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
