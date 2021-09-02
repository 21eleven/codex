use chrono::{DateTime, Local};

type Datetime = DateTime<Local>;

#[derive(Debug)]
pub struct Page {
    pub name: String,
    pub tags: Vec<String>,
    pub links: Vec<String>,
    pub backlinks: Vec<String>,
    pub created: Datetime,
    pub updated: DateTime<Local>,
    pub updates: i64,
}

pub trait Node {
    fn new(name: String) -> Self;
    fn load(path: String) -> Self;
    fn rename(new_name: String);
    fn link(pointing_to: String);
}

impl Node for Page {
    fn new(name: String) -> Page {
        Page {
            name,
            tags: vec![],
            links: vec![],
            backlinks: vec![],
            created: Local::now(),
            updated: Local::now(),
            updates: 1,
        }
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
}
