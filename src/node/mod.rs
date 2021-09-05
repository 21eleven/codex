use chrono::{DateTime, Local};
use log::*;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, create_dir};
use std::io::prelude::*;
use std::path::Path;
use std::io;
use crate::tree;

// type Datetime = DateTime<Local>;



#[derive(Debug, Deserialize, Serialize)]
pub struct Page {
    pub name: String,
    pub tags: Vec<String>,
    pub links: Vec<String>,
    pub backlinks: Vec<String>,
    pub created: DateTime<Local>,
    pub updated: DateTime<Local>,
    pub updates: i64,
}

pub trait Node {
    fn new(name: String) -> Self;
    fn create(path: String, name: String, tags: Option<Vec<String>>) -> tree::Result<Self> where Self: Sized;
    fn load(path: String) -> Self;
    fn rename(new_name: String);
    fn link(pointing_to: String);
    fn tag(&mut self, new_tag: String);
    //fn mark_updated;
}

impl Node for Page {
    fn new(name: String) -> Page {
        let now = Local::now();
        Page {
            name,
            tags: vec![],
            links: vec![],
            backlinks: vec![],
            created: now,
            updated: now,
            updates: 1,
        }
    }
    fn create(path: String, name: String, tags: Option<Vec<String>>) -> tree::Result<Page> {
        let mut node = Node::new(name);
        let n = tree::new_sibling_id(path)?;
        
        Ok(node)
    }
    fn load(path: String) -> Page {
        todo!();
    }
    fn rename(_: String) {
        todo!()
    }
    fn link(pointing_to: String) {
        todo!()
    }
    fn tag(&mut self, new_tag: String) {
        self.tags.push(new_tag);
    }
}
pub fn to_toml(node: Page) -> String {
    toml::to_string_pretty(&node).unwrap()
}

pub fn lay_foundation() {
    fs::create_dir("./codex").unwrap();
    let mut journal: Page = Node::new("journal".to_string());
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
