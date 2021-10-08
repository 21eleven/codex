use async_trait::async_trait;
use git2::Repository;
use log::*;
use nvim_rs::{compat::tokio::Compat, Handler, Neovim};
use std::sync::Arc;

use crate::tree;
use crate::tree::next_sibling_id;
use chrono::Local;
//use tokio::sync::Mutex; // use std::sync::Mutex instead???
use rmpv::Value;
use std::env;
use std::path::PathBuf;
use std::sync::Mutex;
use tokio::io::Stdout;
use tokio::time;

#[derive(Clone)]
pub struct NeovimHandler {
    pub repo: Arc<Mutex<Repository>>,
    pub tree: Arc<Mutex<tree::Tree>>,
}
async fn on_start(nvim: Neovim<Compat<Stdout>>) {
    let yyyymmdd = Local::now().format("%Y%m%d");

    match env::current_dir().unwrap().to_str() {
        Some(dir) => nvim.command(&format!("cd {}/codex", dir)).await.unwrap(),
        None => {}
    }
    nvim.command(&format!("e {}.md", yyyymmdd)).await.unwrap();
    tokio::spawn(async move {
        let mut interval = time::interval(time::Duration::from_millis(250));
        let welcome = "C O D E X 📖".to_string();
        for idx in 1..welcome.len() {
            let s = format!(
                "lua print(\"{}\")",
                welcome.chars().take(idx).collect::<String>()
            );
            interval.tick().await;
            nvim.command(&s).await.unwrap();
        }
    });
}

#[async_trait]
impl Handler for NeovimHandler {
    type Writer = Compat<Stdout>;

    async fn handle_notify(&self, name: String, _args: Vec<Value>, neovim: Neovim<Compat<Stdout>>) {
        match name.as_ref() {
            "start" => {
                log::debug!("starting CODEX!");
                log::debug!("{:?}", self.repo.lock().unwrap().state());
                log::debug!("tree on startup: {}", self.tree.lock().unwrap());
                on_start(neovim).await;
            }
            "ping" => {
                let args_s = format!("{:?}", _args);
                let s = format!("lua print(\"hello pong {}\")", args_s.replace('"', "\\\""));
                neovim.command(s.as_str()).await.unwrap();
            }
            "repeat" => {
                let mut count = 0;
                tokio::spawn(async move {
                    let mut interval = time::interval(time::Duration::from_secs(3));
                    loop {
                        interval.tick().await;
                        let args_s = format!("{:?}", _args);
                        let s = format!(
                            "lua print(\"hello repeat {} {}\")",
                            count,
                            args_s.replace('"', "\\\"")
                        );
                        neovim.command(s.as_str()).await.unwrap();
                        dbg!(count);
                        count += 1;
                    }
                });
            }
            "create" => {
                debug!("{:?}", _args);
                let tree = &mut *self.tree.lock().unwrap();
                tree.create_node(_args);
            }
            "node" => {
                let args: Vec<Option<&str>> = _args.iter().map(|arg| arg.as_str()).collect();
                let tree = &*self.tree.lock().unwrap();
                match args.as_slice() {
                    &[Some(node_ref)] => {
                        debug!(
                            "{:?}: {}",
                            node_ref,
                            tree.nodes.get(&PathBuf::from(&node_ref)).unwrap()
                        );
                    }
                    _ => {}
                }
            }
            "test_sib" => {
                let args: Vec<Option<&str>> = _args.iter().map(|arg| arg.as_str()).collect();
                match args.as_slice() {
                    &[Some(dir)] => {
                        let id = next_sibling_id(&PathBuf::from(dir));
                        debug!("SIB ID: {}", id);
                    }
                    _ => {}
                }
            }
            "stop" => {
                tokio::spawn(async move {
                    let mut interval = time::interval(time::Duration::from_secs(3));
                    interval.tick().await;
                    debug!("woke up, closing");
                });
            }
            _ => {}
        }
    }
    async fn handle_request(
        &self,
        name: String,
        _args: Vec<Value>,
        _neovim: Neovim<Compat<Stdout>>,
    ) -> Result<Value, Value> {
        debug!("in request handler");
        match name.as_str() {
            // "stop" => {
            //     let mut interval = time::interval(time::Duration::from_secs(3));
            //     interval.tick().await;
            //     debug!("woke up, closing");
            //     Ok(Value::Nil)
            // }
            _ => Ok(Value::Nil),
        }
    }
}