use crate::git::stage_all;
use crate::node::{power_of_ten, prepare_path_name, Node, NodeKey, NodeLink};
use chrono::Local;
use git2::Repository;
use log::*;
use nvim_rs::Value;
use regex::Regex;
use std::collections::BTreeMap;
use std::error;
use std::fmt;
use std::fs::{read_to_string, rename, write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

#[derive(Debug)]
pub struct Tree {
    pub nodes: BTreeMap<NodeKey, Node>,
    pub journal: NodeKey,
    pub desk: NodeKey,
    pub dir: PathBuf,
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

/// Used to find the next id for root nodes
pub fn next_sibling_id(key: &PathBuf) -> u64 {
    let metas = WalkDir::new(key)
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

pub fn next_child_id(path: &str) -> u64 {
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
    pub fn load(self: &mut Tree) {
        fn dfs(
            name: Option<NodeKey>,
            node_map: &mut BTreeMap<NodeKey, Node>,
            parent: Option<NodeKey>,
            journal: &mut Option<NodeKey>,
            desk: &mut Option<NodeKey>,
            base: &Path,
        ) {
            let n = base.to_str().unwrap().chars().count() + 1; // plus one to factor the / char
            let search_dir = match name.clone() {
                None => base.to_path_buf(),
                Some(name_path) => base.join(PathBuf::from(name_path)),
            };
            dbg!(&base, &search_dir);
            debug!("{:?} {:?}", &base, &search_dir);
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
                        // .file_name()
                        // .unwrap()
                        .to_str()
                        .unwrap()
                        // .to_string()
                        .chars()
                        .skip(n)
                        .collect::<String>()
                })
                .collect::<Vec<NodeKey>>();
            dbg!(&children);
            dbg!(&base);
            for node in children.iter() {
                dfs(
                    Some(node.clone()),
                    node_map,
                    name.clone(),
                    journal,
                    desk,
                    &base,
                );
            }
            match name {
                Some(namepath) => {
                    // debug!("{:?}", &namepath);
                    let meta_path = base.join(&namepath).join("meta.toml");
                    let node = Node::from_tree(
                        namepath,
                        &meta_path,
                        parent,
                        children,
                        base.to_str().unwrap(),
                    );
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
            &self.dir,
        );
        self.nodes = node_map;
        self.journal = journal.unwrap();
        self.desk = desk.unwrap();
    }
    pub fn build(root: &str) -> Result<Tree> {
        let mut node_map: BTreeMap<NodeKey, Node> = BTreeMap::new();
        dbg!(root);
        assert_ne!(root.chars().last().unwrap(), '/');
        Ok(Tree {
            nodes: node_map,
            journal: NodeKey::new(),
            desk: NodeKey::new(),
            dir: PathBuf::from(root),
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
            let today = self
                .create_node(Some(&journal), Some(&today.to_string()))
                .unwrap();
            let node = self.nodes.get(&journal).unwrap();
            let yesterday = newest_child;
            rollover_todos_from_yesterday(&yesterday, &today);
            today
        }
    }
    /// Validates data from RPC call
    /// TODO: move into a module response for linking nvim RPC calls and backend
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
    /// Create a node within Codex
    pub fn create_node(&mut self, parent: Option<&str>, child: Option<&str>) -> Result<NodeKey> {
        // TODO: what would happen if the input had a '/'? sanatize
        // need to decouple this node creation on tree
        // from processing of RPC message pack value
        match (parent, child) {
            (Some(parent), Some(child)) => {
                let parent = parent.to_string();
                debug!("parent {:?} and child {:?}", parent, child);
                dbg!(&parent, &child);
                let child = match self.nodes.get_mut(&parent) {
                    Some(parent) => {
                        Some(parent.create_child(child.to_string(), self.dir.to_str().unwrap()))
                    }
                    None => {
                        error!("no node in tree named: {:?}", parent);
                        None
                    }
                };
                dbg!(&child);
                if let Some(child) = child {
                    // a new node is created, it has a parent
                    let child_id = child.id.clone();
                    self.nodes.insert(child.id.clone(), child);
                    // stage_all().unwrap();
                    let parent_ref = get_parent(&child_id).unwrap();
                    if let Some(n) = power_of_ten(get_node_key_number(&child_id)) {
                        // this newly created node is a power of 10 node
                        // we must go to all the siblings and rename them
                        // TODO make this section more DRY
                        let _repo = Repository::open(self.dir.to_str().unwrap()).unwrap();
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
                            let old_path = node_clone.directory.join(&sibid);
                            let new_path = node_clone.directory.join(&newid);
                            // move sib node on fs
                            rename(old_path, new_path).unwrap();
                            // link is another node
                            // that this node points to in its content
                            for (id, link) in &node_clone.links {
                                let linked = self.nodes.get_mut(&link.node).unwrap();
                                linked.rename_backlink(&(id.clone(), link.timestamp), &newid);
                            }
                            // a backlink is a node that has a link
                            // in its content that points to this node
                            for (id, backlink) in &node_clone.backlinks {
                                let backlinked = self.nodes.get_mut(&backlink.node).unwrap();
                                backlinked.rename_link(&id.0, &newid);
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
                                // remove child from map
                                let mut node = map.remove(node_ref).unwrap();
                                let (_, node_name) = node_ref.rsplit_once('/').unwrap();
                                // calc new name
                                let newid = format!("{}/{}", parent, node_name,);
                                // inform links
                                for (id, link) in &node.links {
                                    let linked = map.get_mut(&link.node).unwrap();
                                    linked.rename_backlink(&(id.clone(), link.timestamp), &newid);
                                }
                                for (id, backlink) in &node.backlinks {
                                    let backlinked = map.get_mut(&backlink.node).unwrap();
                                    backlinked.rename_link(&id.0, &newid);
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
                        // commit_paths(
                        //     &repo,
                        //     vec![Path::new("./*")],
                        //     &format!("node renames due to new power of ten node {}", child_id),
                        // )
                        // .unwrap();
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
                // should have a tree function for getting root siblings
                let node = Node::create(node_name.to_string(), None, self.dir.to_str().unwrap());
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
    pub fn link(
        &mut self,
        text: &str,
        from: &str,
        from_line: u64,
        from_char: u64,
        to: &str,
        to_line: u64,
        to_char: u64,
    ) {
        let (link, backlink) = NodeLink::pair(
            text.to_string(),
            from.to_string(),
            from_line,
            from_char,
            to.to_string(),
            to_line,
            to_char,
        );
        self.nodes.get_mut(from).unwrap().insert_link(link);
        self.nodes.get_mut(to).unwrap().insert_backlink(backlink);
    }
    pub fn get_link(&self, name: &str, text: &str) -> (NodeKey, u64) {
        let node = self.nodes.get(name).unwrap();
        node.get_link(text)
    }
    pub fn latest_journal(&self) -> NodeKey {
        let journal_node = self.nodes.get(&self.journal).unwrap();
        journal_node.children[journal_node.children.len() - 1].clone()
    }
    pub fn next_sibling(&self, node: &str, previous: bool) -> NodeKey {
        let child = self.nodes.get(node).unwrap();
        let parent_key = match &child.parent {
            None => return child.id.clone(), // TODO handle when node is on bottom level
            Some(parent) => parent,
        };
        let parent = self.nodes.get(parent_key).unwrap();
        let family_size = parent.children.len();
        let index = child.index();
        let mut sibling_index = if previous { index - 1 } else { index + 1 };
        sibling_index %= family_size;
        if sibling_index == 0 {
            sibling_index = family_size;
        }
        // nodes are 1 indexed in the tree heirarchy
        // the children vec is zero indexed
        parent.children[sibling_index - 1].clone()
    }
    pub fn nodes_by_recency(&self) -> Vec<&Node> {
        let mut nodes = self.nodes.values().collect::<Vec<&Node>>();
        nodes.sort_unstable_by(|a, b| b.updated.cmp(&a.updated));
        nodes
    }
}

fn rollover_todos_from_yesterday(yesterday: &NodeKey, today: &NodeKey) {
    let yesterday_body = read_to_string(Path::new(yesterday).join("_.md")).unwrap();
    let today_file = Path::new(today).join("_.md");
    let today_body = read_to_string(&today_file).unwrap();
    write(today_file, move_todos(yesterday_body, today_body)).unwrap();
}
fn move_todos(prior: String, current: String) -> String {
    let todos: String = Regex::new(r"(?m)^( |\t)*- \[\] .*")
        .unwrap()
        .find_iter(&prior)
        .map(|x| x.as_str().to_string())
        .collect::<Vec<String>>()
        .join("\n");
    format!("{}\n{}\n", current, todos)
}
