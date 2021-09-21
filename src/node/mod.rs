use crate::tree::{self, new_sibling_id};
use chrono::{DateTime, Local};
use log::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::convert::TryInto;
use std::fs::{self, create_dir, read_to_string, File};
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

// type Datetime = DateTime<Local>;
// struct HierarchicalIdentifier {
//     codex_path: String
// }
const CODEX_ROOT: &str = "./codex/";
pub enum Entry {
    Page,
    Todo,
}

type Entity = Box<Node>;

pub type NodeRef = PathBuf;

#[derive(Debug)]
pub struct Node {
    id: NodeRef,
    name: String,
    parent: Option<NodeRef>,
    siblings: Vec<NodeRef>, // all siblings should have a pointer to the same vec // or HierarchicalIdentifiers?
    children: Vec<NodeRef>, // parent has a point to it's children shared/sibling/family vec
    links: Vec<NodeRef>,
    backlinks: Vec<NodeRef>,
    tags: HashSet<String>,
    created: DateTime<Local>,
    updated: DateTime<Local>,
    updates: u64,
}

fn prepare_path_name(node_name: &String) -> String {
    node_name
        .to_ascii_lowercase()
        .chars()
        .map(|c| match c {
            ' ' => '-',
            _ => c,
        })
        .collect()
}

impl Node {
    fn new(path: String, name: String, parent: Option<Node>) -> Node {
        let path_name = prepare_path_name(&name);
        let (node_path, parent_option) = match parent {
            Some(parent_node) => todo!(),
            None => {
                let path = PathBuf::from("");
                let sibling_num = new_sibling_id(&path);
                (
                    path.join(PathBuf::from(format!("{}-{}/", sibling_num, path_name))),
                    None,
                )
            }
        };
        let now = Local::now();
        Node {
            id: node_path,
            name,
            parent: parent_option,
            siblings: vec![],
            children: vec![],
            links: vec![],
            backlinks: vec![],
            tags: HashSet::new(),
            created: now,
            updated: now,
            updates: 1,
        }
    }
    pub fn from_tree(
        id: PathBuf,
        toml_path: &Path,
        parent: Option<NodeRef>,
        siblings: Vec<NodeRef>,
        children: Vec<NodeRef>,
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
    pub fn write(&mut self) {
        todo!();
    }
    pub fn tick_update(&mut self) {
        let now = Local::now();

        if now.date() != self.updated.date() {
            self.updates += 1;
        }
        self.updated = now;
    }
    pub fn create_child(&mut self) {
        todo!();
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
    fn tag(&mut self, new_tag: String) {
        self.tags.push(new_tag);
    }
}
pub fn to_toml(node: NodeMeta) -> String {
    toml::to_string_pretty(&node).unwrap()
}

mod codex_date_format {
    use chrono::{DateTime, Local, TimeZone};
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &'static str = "%Y-%m-%dT%H:%M:%S%z"; // or `%:z`?

    pub fn serialize<S>(date: &DateTime<Local>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Local>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Local
            .datetime_from_str(&s, FORMAT)
            .map_err(serde::de::Error::custom)
    }
}

pub fn lay_foundation() {
    fs::create_dir("./codex").unwrap();
    let mut journal: NodeMeta = NodeMeta::new("journal".to_string());
    journal.tag("journal".to_string());
    let journal_root_path = Path::new("codex/1-journal");
    create_dir(journal_root_path).unwrap();
    let data = journal_root_path.join("_.md");
    let metadata = journal_root_path.join("meta.toml");
    let display = journal_root_path.display();
    let mut file = match File::create(metadata.as_path()) {
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };
    let journal_toml = to_toml(journal);

    match file.write_all(journal_toml.as_str().as_bytes()) {
        Err(why) => panic!("couldn't write to {}: {}", display, why),
        Ok(_) => debug!("successfully wrote to {}", display),
    }

    let mut file = match File::create(data.as_path()) {
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };
    match file.write_all("".as_bytes()) {
        Err(why) => panic!("couldn't write to {}: {}", display, why),
        Ok(_) => debug!("successfully wrote to {}", display),
    }
}
