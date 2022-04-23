use crate::nvim::Telescoped;
use crate::tree::next_sibling_id;
use chrono::{DateTime, Local};
use log::*;
use nvim_rs::Value;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{create_dir, read_to_string, File, OpenOptions};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
mod date_serde;
use date_serde::codex_date_format;
mod utils;
use crate::git::commit_paths;
use git2::Repository;
use serde_derive;
use std::fmt;
pub use utils::*;

#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeKey,
    pub name: String,
    pub display_name: String,
    pub parent: Option<NodeKey>,
    pub children: Vec<NodeKey>, // parent has a point to it's children shared/sibling/family vec
    pub links: HashMap<String, NodeLink>,
    pub backlinks: HashMap<(String, i64), NodeLink>,
    pub tags: HashSet<String>,
    pub internal: HashSet<String>,
    pub created: DateTime<Local>,
    pub updated: DateTime<Local>,
    pub updates: u64,
    directory: PathBuf,
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Node({}): {{", self.name)?;
        writeln!(f, "\t id: {}", self.id)?;
        writeln!(
            f,
            "\t parent: {}",
            match &self.parent {
                Some(parent) => parent,
                None => "None",
            }
        )?;
        writeln!(f, "\t children: {:?}", self.children)?;
        writeln!(f, "\t links: {:?}", self.links)?;
        writeln!(f, "\t backlinks: {:?}", self.backlinks)?;
        writeln!(f, "\t tags: {:?}", self.tags)?;
        writeln!(f, "}}")?;
        Ok(())
    }
}

impl Node {
    fn new(name: String, parent: Option<&Node>, directory: PathBuf) -> Node {
        let path_name = prepare_path_name(&name);
        let (node_key, parent_option) = match parent {
            Some(parent_node) => {
                let sibling_num = parent_node.children.len() + 1;
                let node_key = format!("{}/{}-{}", parent_node.id, sibling_num, path_name);
                (node_key, Some(parent_node.id.clone()))
            }
            None => {
                let sibling_num = next_sibling_id(&directory);
                // TODO: are we handling order of mag rollover here?
                (format!("{}-{}", sibling_num, path_name), None)
            }
        };
        let now = Local::now();
        Node {
            display_name: format_display_name(&node_key),
            id: node_key,
            name,
            parent: parent_option,
            children: vec![],
            links: HashMap::new(),
            backlinks: HashMap::new(),
            tags: HashSet::new(),
            internal: HashSet::new(),
            created: now,
            updated: now,
            updates: 1,
            directory,
        }
    }
    /// Create files for a node outside of a node tree
    /// Used to boot strap initial codex directory layout
    pub fn create(name: String, parent: Option<&Node>, path: &str) -> Node {
        // what if directory already exists?
        let node = Node::new(name.clone(), parent, PathBuf::from(path));
        let directory = Path::new(path).join(&node.id);
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
        match file.write_all(format!("# {}\n", name).as_bytes()) {
            Err(why) => panic!("couldn't write to {}: {}", display, why),
            Ok(_) => debug!("successfully wrote to {}", display),
        }
        // stage new node/directory in git repo
        // debug!("STAGING: {:?}", &directory.join("*"));
        // stage_paths(vec![&directory.join("*")]).unwrap();
        node
    }
    pub fn from_tree(
        id: NodeKey,
        toml_path: &Path,
        parent: Option<NodeKey>,
        children: Vec<NodeKey>,
        directory: &str,
    ) -> Node {
        // let (name, tags, links, backlinks, created, updated, updates) =
        //     NodeMeta::from_toml(toml_path).data();
        let metadata = NodeMeta::from_toml(toml_path);
        Node {
            display_name: format_display_name(&id),
            id,
            name: metadata.name,
            parent,
            children,
            links: metadata
                .links
                .into_iter()
                .map(|s| NodeLink::parse_link_w_key(s))
                .collect(),
            backlinks: metadata
                .backlinks
                .into_iter()
                .map(|s| NodeLink::parse_backlink_w_key(s))
                .collect(),
            tags: metadata.tags.into_iter().collect(),
            internal: metadata.internal.into_iter().collect(),
            created: metadata.created,
            updated: metadata.updated,
            updates: metadata.updates,
            directory: PathBuf::from(directory),
        }
    }
    pub fn index(&self) -> usize {
        let path = match self.id.rsplit_once('/') {
            Some((_, node)) => node,
            None => self.id.as_str(),
        };
        let (num, name) = path.split_once('-').unwrap();
        num.parse::<usize>().unwrap()
    }
    pub fn create_child(&mut self, name: String, path: &str) -> Node {
        let child = Node::create(name, Some(self), path);
        self.children.push(child.id.clone());
        self.tick_update_and_write_meta();
        child
    }
    fn tag(&mut self, new_tag: String) {
        self.tags.insert(new_tag);
        self.tick_update_and_write_meta();
    }
    pub fn insert_link(&mut self, link: NodeLink) {
        self.links.insert(link.text.clone(), link);
        self.tick_update_and_write_meta();
    }
    pub fn insert_backlink(&mut self, backlink: NodeLink) {
        self.backlinks
            .insert((backlink.text.clone(), backlink.timestamp), backlink);
        self.tick_update_and_write_meta();
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
    pub fn rename_link(&mut self, old_name: &str, new_name: &str) {
        // TODO rename all instances of the link in the content file
        // for i in 0..self.links.len() {
        // should links be a hashset?
        // if self.links[i] == *old_name {
        // self.links[i] = new_name.clone().to_path_buf();
        // break
        // }
        // }
        // self.links.remove(old_name);
        // self.links.insert(new_name.to_string());
        // self.write_meta();
        todo!()
    }
    pub fn rename_backlink(&mut self, old_name: &str, new_name: &str) {
        // self.backlinks.remove(old_name);
        // self.backlinks.insert(new_name.to_string());
        // self.write_meta();
        todo!()
    }
    pub fn metadata_path(&self) -> PathBuf {
        self.directory.join(&self.id).join("meta.toml")
    }

    pub fn write_meta(&self) {
        let metadata = self.metadata_path();
        let meta_toml = NodeMeta::from(self).to_toml();
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
    pub fn tick_update_and_write_meta(&mut self) {
        let now = Local::now();

        if now.date() != self.updated.date() {
            self.updates += 1;
        }
        self.updated = now;
        self.write_meta();
    }
}
impl Telescoped for Node {
    fn entry(&self) -> Value {
        Value::Map(vec![
            (
                Value::String("id".into()),
                Value::String(self.id.clone().into()),
            ),
            (
                Value::String("display".into()),
                Value::String(self.display_name.clone().into()),
            ),
        ])
    }
}

pub type NodeKey = String;

impl Telescoped for NodeKey {
    fn entry(&self) -> Value {
        Value::Map(vec![
            (
                Value::String("id".into()),
                Value::String(self.clone().into()),
            ),
            (
                Value::String("display".into()),
                Value::String(format_display_name(self).into()),
            ),
        ])
    }
}

// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
// enum LinkType {
//     Link(String),
//     BackLink(String),
// }
// use LinkType::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeLink {
    pub node: NodeKey,
    pub text: String,
    pub timestamp: i64,
    pub line: u64,
    pub char: u64,
    pub is_name_linked: bool,
}

impl fmt::Display for NodeLink {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[[ {}| -> [{}]({}|{}) ]]",
            self.text, self.node, self.line, self.char
        )?;
        Ok(())
    }
}

impl NodeLink {
    pub fn pair(
        text: String,
        from: String,
        from_line: u64,
        from_char: u64,
        to: String,
        to_line: u64,
        to_char: u64,
    ) -> (Self, Self) {
        let timestamp = chrono::Utc::now().timestamp();
        let is_name_linked = link_text_is_node_name(&text, &to);
        (
            NodeLink {
                node: to,
                text: text.clone(),
                timestamp,
                line: to_line,
                char: to_char,
                is_name_linked,
            },
            NodeLink {
                node: from,
                text,
                timestamp,
                line: from_line,
                char: from_char,
                is_name_linked,
            },
        )
    }
    pub fn to_toml(&self) -> String {
        // there should be some kind of string escaping here...
        // to check that the link text doesn't have `|,|` in it

        let link_varient = if self.is_name_linked {
            "name_ref"
        } else {
            "text_ref"
        };
        format!(
            "{}|,|{}|,|{}|,|{}|,|{}|,|{link_varient}",
            self.text, self.timestamp, self.node, self.line, self.char
        )
    }
    fn from_toml(toml: String) -> NodeLink {
        let (text, link) = toml.split_once("|,|").unwrap();
        let (timestamp, link) = link.split_once("|,|").unwrap();
        let (node, link) = link.split_once("|,|").unwrap();
        let (line, link) = link.split_once("|,|").unwrap();
        let (char, link_varient) = link.split_once("|,|").unwrap();
        NodeLink {
            node: node.to_string(),
            text: text.to_string(),
            timestamp: timestamp.parse::<i64>().unwrap(),
            line: line.parse::<u64>().unwrap(),
            char: char.parse::<u64>().unwrap(),
            is_name_linked: match link_varient {
                "name_ref" => true,
                _ => false,
            },
        }
    }
    pub fn parse_link_w_key(toml: String) -> (String, NodeLink) {
        let link = Self::from_toml(toml);
        (link.text.clone(), link)
    }
    pub fn parse_backlink_w_key(toml: String) -> ((String, i64), NodeLink) {
        let link = Self::from_toml(toml);
        ((link.text.clone(), link.timestamp), link)
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
    pub internal: Vec<String>,
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
            internal: vec![],
        }
    }
    pub fn from(node: &Node) -> NodeMeta {
        let mut tags: Vec<String> = node.tags.clone().into_iter().collect();
        tags.sort_unstable();
        let mut internal: Vec<String> = node.internal.clone().into_iter().collect();
        internal.sort_unstable();
        NodeMeta {
            name: node.name.clone(),
            tags,
            links: node.links.values().map(|link| link.to_toml()).collect(),
            backlinks: node.backlinks.values().map(|link| link.to_toml()).collect(),
            created: node.created,
            updated: node.updated,
            updates: node.updates,
            internal,
        }
    }
    pub fn from_toml(toml_path: &Path) -> NodeMeta {
        let toml_string = read_to_string(toml_path).unwrap();
        toml::from_str(&toml_string).unwrap()
    }
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).unwrap()
    }
}

pub fn init_codex_repo(path: Option<&str>) -> Repository {
    let path = path.unwrap_or("./");
    let repo = Repository::init(path).unwrap();
    let mut journal = Node::create("journal".to_string(), None, path);
    journal.tag(String::from("journal"));
    journal.write_meta();
    dbg!(journal);
    let mut desk = Node::create("desk".to_string(), None, path);
    desk.tag(String::from("desk"));
    desk.write_meta();
    repo
}
