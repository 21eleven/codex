use chrono::{Local , DateTime};

type Datetime = DateTime<Local>; 

pub struct Node {
    pub name: String,
    pub tags: Vec<String>,
    pub links: Vec<String>,
    pub backlinks: Vec<String>,
    pub created: Datetime,
    pub updated: DateTime<Local>,
    pub updates: i64,
}
