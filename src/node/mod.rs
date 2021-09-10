use crate::tree::{self, new_sibling_id};
use chrono::{DateTime, Local};
use log::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, create_dir, File};
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

pub struct Node {
    id: PathBuf,
    name: String,
    parent: Option<Box<Node>>,
    siblings: Box<Vec<String>>, // all siblings should have a pointer to the same vec // or HierarchicalIdentifiers?
    children: Vec<String>,      // parent has a point to it's children shared/sibling/family vec
    links: Vec<Box<Node>>,
    backlinks: Vec<Box<Node>>,
    tags: HashSet<String>,
    created: DateTime<Local>,
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
        Node {
            id: node_path,
            name,
            parent: parent_option,
            siblings: Box::new(vec![]),
            children: vec![],
            links: vec![],
            backlinks: vec![],
            tags: HashSet::new(),
            created: Local::now(),
            updates: 1,
        }
    }
    //     fn init(path: String, name: String, ntype: Entry) ->Node {
    //         match ntype {
    //             Entry::Page => {
    //                 let meta_string = to_toml(PageMeta::new(name));
    //
    //
    //                 Node {
    //
    //                 }
    //
    //             }
    //             _ => todo!()
    //
    //         }
    //     }
    //     fn create(path: String, name: String, tags: Option<Vec<String>>) -> tree::Result<PageMeta> {
    //         let mut node = PageMeta::new(name);
    //         let n = tree::new_sibling_id(path)?;
    //
    //         Ok(node)
    //     }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NodeMeta {
    pub name: String,
    pub tags: Vec<String>,
    pub links: Vec<String>,
    pub backlinks: Vec<String>,
    pub created: DateTime<Local>,
    pub updated: DateTime<Local>,
    pub updates: i64,
}

// pub trait NodeMeta {
//     fn new(name: String) -> Self;
//     fn create(path: String, name: String, tags: Option<Vec<String>>) -> tree::Result<Self> where Self: Sized;
//     fn load(path: String) -> Self;
//     fn rename(new_name: String);
//     fn link(pointing_to: String);
//     fn tag(&mut self, new_tag: String);
//     //fn mark_updated;
// }

impl NodeMeta {
    fn new(name: String) -> NodeMeta {
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
// fn create(path: String, name: String, tags: Option<Vec<String>>) -> tree::Result<PageMeta> {
//     let mut node = PageMeta::new(name);
//     let n = tree::new_sibling_id(path)?;
//
//     Ok(node)
// }
//     fn load(path: String) -> PageMeta {
//         todo!();
//     }
//     fn rename(_: String) {
//         todo!()
//     }
//     fn link(pointing_to: String) {
//         todo!()
//     }
    fn tag(&mut self, new_tag: String) {
        self.tags.push(new_tag);
    }
}
pub fn to_toml(node: NodeMeta) -> String {
    toml::to_string_pretty(&node).unwrap()
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
