use crate::tree::{self, next_sibling_id, Tree};
use chrono::{DateTime, Local};
use log::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::convert::TryInto;
use std::fs::{self, create_dir, read_to_string, File, OpenOptions};
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
mod date_serde;
use crate::utils::commit_paths;
use date_serde::codex_date_format;
use git2::Repository;
use std::fmt;

// type Datetime = DateTime<Local>;
// struct HierarchicalIdentifier {
//     codex_path: String
// }
const CODEX_ROOT: &str = "./codex/";
pub enum Entry {
    Page,
    Todo,
}

pub fn power_of_ten(mut n: u64) -> Option<u64> {
    let mut pow = 1;
    let mut r = 0;
    loop {
        if r > 0 || n == 0 {
            return None;
        } else if n == 10 {
            return Some(pow);
        } else {
            pow += 1;
            r = n % 10;
            n /= 10;
        }
    }
}

type Entity = Box<Node>;

pub type NodeRef = PathBuf;
pub type NodeKey = String;

#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeKey,
    pub name: String,
    pub parent: Option<NodeKey>,
    pub siblings: Vec<NodeKey>, // all siblings should have a pointer to the same vec // or HierarchicalIdentifiers?
    pub children: Vec<NodeKey>, // parent has a point to it's children shared/sibling/family vec
    pub links: HashSet<NodeKey>,
    pub backlinks: HashSet<NodeKey>,
    pub tags: HashSet<String>,
    pub created: DateTime<Local>,
    pub updated: DateTime<Local>,
    pub updates: u64,
}
impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Node({}): {{\n", self.name)?;
        write!(f, "\t id: {}\n", self.id)?;
        write!(
            f,
            "\t parent: {}\n",
            match &self.parent {
                Some(parent) => parent,
                None => "None",
            }
        )?;
        write!(f, "\t siblings: {:?}\n", self.siblings)?;
        write!(f, "\t children: {:?}\n", self.children)?;
        write!(f, "\t links: {:?}\n", self.links)?;
        write!(f, "\t backlinks: {:?}\n", self.backlinks)?;
        write!(f, "\t tags: {:?}\n", self.tags)?;
        write!(f, "}}\n")?;
        Ok(())
    }
}
pub fn prepare_path_name(node_name: &String) -> String {
    node_name
        // .to_ascii_lowercase()
        .chars()
        .map(|c| match c {
            ' ' => '-',
            _ => c,
        })
        .collect()
}

impl Node {
    fn new(name: String, parent: Option<&Node>) -> Node {
        let path_name = prepare_path_name(&name);
        let (node_key, parent_option) = match parent {
            Some(parent_node) => {
                let sibling_num = parent_node.children.len() + 1;
                let node_key = format!("{}/{}-{}", parent_node.id, sibling_num, path_name);
                (node_key, Some(parent_node.id.clone()))
            }
            None => {
                let sibling_num = next_sibling_id(&"".to_string());
                // TODO: are we handling order of mag rollover here?
                (format!("{}-{}", sibling_num, path_name), None)
            }
        };
        let now = Local::now();
        Node {
            id: node_key,
            name,
            parent: parent_option,
            siblings: vec![],
            children: vec![],
            links: HashSet::new(),
            backlinks: HashSet::new(),
            tags: HashSet::new(),
            created: now,
            updated: now,
            updates: 1,
        }
    }
    pub fn create(name: String, parent: Option<&Node>) -> Node {
        // what if directory already exists?
        let node = Node::new(name, parent);
        let directory = Path::new("codex").join(&node.id);
        let meta_toml = NodeMeta::from(&node).to_toml();
        create_dir(&directory).unwrap();
        let data = directory.join("_.md");
        let metadata = directory.join("meta.toml");
        let display = metadata.display();
        let mut file = match File::create(metadata.as_path()) {
            Err(why) => panic!("couldn't create {}: {}", display, why),
            Ok(file) => file,
        };
        match file.write_all(meta_toml.as_str().as_bytes()) {
            Err(why) => panic!("couldn't write to {}: {}", display, why),
            Ok(_) => debug!("successfully wrote to {}", display),
        }
        let display = data.display();
        let mut file = match File::create(data.as_path()) {
            Err(why) => panic!("couldn't create {}: {}", display, why),
            Ok(file) => file,
        };
        match file.write_all("".as_bytes()) {
            Err(why) => panic!("couldn't write to {}: {}", display, why),
            Ok(_) => debug!("successfully wrote to {}", display),
        }
        node
    }
    pub fn from_tree(
        id: NodeKey,
        toml_path: &Path,
        parent: Option<NodeKey>,
        siblings: Vec<NodeKey>,
        children: Vec<NodeKey>,
    ) -> Node {
        let (name, tags, links, backlinks, created, updated, updates) =
            NodeMeta::from_toml(toml_path).data();
        Node {
            id,
            name,
            parent,
            siblings,
            children,
            links: links.into_iter().map(|p| p.try_into().unwrap()).collect(),
            backlinks: backlinks
                .into_iter()
                .map(|p| p.try_into().unwrap())
                .collect(),
            tags: tags.into_iter().collect(),
            created,
            updated,
            updates,
        }
    }
    pub fn rerank(&mut self, rank: u64) {
        todo!();
    }
    pub fn mv(&mut self, new_path: NodeKey) {
        // should probably return a result
        // primitive fn for moving across fs
        // should be a git move
        self.id = new_path;
        todo!();
    }
    pub fn rename_link(&mut self, old_name: &NodeKey, new_name: &NodeKey) {
        // TODO rename all instances of the link in the content file
        // for i in 0..self.links.len() {
        // should links be a hashset?
        // if self.links[i] == *old_name {
        // self.links[i] = new_name.clone().to_path_buf();
        // break
        // }
        // }
        self.links.remove(old_name);
        self.links.insert(new_name.to_string());
        self.write_meta();
    }
    pub fn rename_backlink(&mut self, old_name: &NodeKey, new_name: &NodeKey) {
        self.backlinks.remove(old_name);
        self.backlinks.insert(new_name.to_string());
        self.write_meta();
    }
    pub fn update(&mut self) {
        self.tick_update();
        self.write()
    }
    pub fn write(&mut self) {
        todo!();
    }
    pub fn write_meta(&self) {
        let metadata = Path::new("codex").join(&self.id).join("meta.toml");
        let meta_toml = NodeMeta::from(&self).to_toml();
        let display = metadata.display();
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(metadata.as_path())
            .unwrap();
        match file.write_all(meta_toml.as_str().as_bytes()) {
            Err(why) => panic!("couldn't write to {}: {}", display, why),
            Ok(_) => debug!("successfully wrote to {}", display),
        }
    }
    pub fn tick_update(&mut self) {
        let now = Local::now();

        if now.date() != self.updated.date() {
            self.updates += 1;
        }
        self.updated = now;
    }
    pub fn create_child(&mut self, name: String) -> Node {
        let child = Node::create(name, Some(&self));
        self.children.push(child.id.clone());
        child
    }
    fn tag(&mut self, new_tag: String) {
        self.tags.insert(new_tag);
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NodeMeta {
    pub name: String,
    pub tags: Vec<String>,
    pub links: Vec<String>,
    pub backlinks: Vec<String>,
    #[serde(with = "codex_date_format")]
    pub created: DateTime<Local>,
    #[serde(with = "codex_date_format")]
    pub updated: DateTime<Local>,
    pub updates: u64,
}

impl NodeMeta {
    pub fn new(name: String) -> NodeMeta {
        let now = Local::now();
        NodeMeta {
            name,
            tags: vec![],
            links: vec![],
            backlinks: vec![],
            created: now,
            updated: now,
            updates: 1,
        }
    }
    pub fn from(node: &Node) -> NodeMeta {
        let mut tags: Vec<String> = node.tags.clone().into_iter().collect();
        tags.sort_unstable();
        let mut links: Vec<String> = node.links.clone().into_iter().collect();
        links.sort_unstable();
        let mut backlinks: Vec<String> = node.backlinks.clone().into_iter().collect();
        backlinks.sort_unstable();
        NodeMeta {
            name: node.name.clone(),
            tags,
            links,
            backlinks,
            created: node.created,
            updated: node.updated,
            updates: node.updates,
        }
    }
    pub fn from_toml(toml_path: &Path) -> NodeMeta {
        let toml_string = read_to_string(toml_path).unwrap();
        toml::from_str(&toml_string).unwrap()
    }
    pub fn data(
        self,
    ) -> (
        String,
        Vec<String>,
        Vec<String>,
        Vec<String>,
        DateTime<Local>,
        DateTime<Local>,
        u64,
    ) {
        (
            self.name,
            self.tags,
            self.links,
            self.backlinks,
            self.created,
            self.updated,
            self.updates,
        )
    }
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).unwrap()
    }
}

pub fn init_codex_repo() -> Repository {
    fs::create_dir(CODEX_ROOT).unwrap();
    let repo = Repository::init("./").unwrap();
    let mut journal = Node::create("journal".to_string(), None);
    journal.tag(String::from("journal"));
    journal.write_meta();
    debug!("created journal: {}", journal);
    let mut desk = Node::create("desk".to_string(), None);
    desk.tag(String::from("desk"));
    desk.write_meta();
    debug!("created desk: {}", desk);
    commit_paths(&repo, vec![&Path::new("codex/*")], "codex init").unwrap();
    debug!("codex git repo initialized");
    repo
}
