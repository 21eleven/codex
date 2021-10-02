use async_trait::async_trait;
use chrono::Local;
use git2::Repository;
use log::*;
use nvim_rs::{compat::tokio::Compat, create::tokio as create, Handler, Neovim};
use rmpv::Value;
use std::default::Default;
use std::env;
use std::error::Error;
use std::sync::Arc;
use tokio::io::Stdout;
//use tokio::sync::Mutex; // use std::sync::Mutex instead???
use std::sync::Mutex;
use tokio::time;
mod node;
mod tree;

use node::{lay_foundation, Node};
use nom::{
    bytes::complete::{tag, take_till},
    character::complete::char,
    IResult,
};
use std::path::PathBuf;

#[derive(Clone)]
struct NeovimHandler {
    repo: Arc<Mutex<Repository>>,
    tree: Arc<Mutex<tree::Tree>>,
}
fn parse_name(input: &str) -> IResult<&str, &str> {
    let (input, _) = char('"')(input)?;
    take_till(|c| c == '"')(input)
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
                log::debug!("tree on startup: {:?}", self.tree.lock().unwrap());
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
                let args: Vec<Option<&str>> = _args.iter().map(|arg| arg.as_str()).collect();
                match args.as_slice() {
                    &[Some(parent), Some(child)] => {
                        let parent = PathBuf::from(parent);
                        debug!("parent {:?} and child {:?}", parent, child);
                        let tree = &mut *self.tree.lock().unwrap();
                        let child_id = match tree.nodes.get_mut(&parent) {
                            Some(parent) => {
                                let child = parent.create_child(child.to_string());
                                let child_id = child.id.clone();
                                tree.nodes.insert(child.id.clone(), child);
                                Some(child_id)
                            }
                            None => {
                                error!("no node in tree named: {:?}", parent);
                                None
                            }
                        };
                        if let Some(id) = child_id {
                            debug!(
                                "parent in tree: {:?}",
                                tree.nodes.get(id.clone().parent().unwrap())
                            );
                            debug!(
                                "child in tree: {:?}",
                                tree.nodes.get(&id)
                            )
                        }
                    }
                    &[Some(child)] => {
                        debug!("single child {:?}", child);
                        for c in child.chars() {
                            debug!("{:?}", c);
                        }
                    }
                    _ => {
                        error!("invalid args to create: {:?}", _args);
                    }
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

#[tokio::main]
async fn main() {
    let plugin_dir = if let Ok(dir) = std::env::var("CODEX_HOME") {
        dir
    } else {
        let error_msg = "ENV var CODEX_HOME not set, panicking";
        std::panic::panic_any(error_msg);
    };
    let config_file = format!("{}/codex-log.toml", plugin_dir);

    log_panics::init();
    if let Err(e) = log4rs::init_file(format!("{}/codex-log.toml", plugin_dir), Default::default())
    {
        eprintln!("Error configuring logging with {}: {:?}", config_file, e);
        return;
    }
    debug!("backend live within: {:?}", env::current_dir().unwrap());
    let repo = Arc::new(Mutex::new(match Repository::open("./") {
        Ok(repo) => repo,
        Err(_) => {
            lay_foundation();
            Repository::init("./").unwrap()
        }
    }));
    let tree = Arc::new(Mutex::new(
        match tree::Tree::build("./codex/".to_string()) {
            Ok(tree) => {
                debug!("tree gud!");
                tree
            }
            Err(e) => {
                error!("tree ERROR! {:?}", e);
                panic!("tree Error - PANIC {:?}", e);
            }
        },
    ));
    let handler = NeovimHandler { repo, tree };
    let (nvim, io_handler) = create::new_parent(handler).await;
    match io_handler.await {
        Err(join_error) => {
            error!("Error joining IO loop: {}", join_error);
        }
        Ok(Err(error)) => {
            if !error.is_reader_error() {
                nvim.err_writeln(&format!("Error: {}", error))
                    .await
                    .unwrap_or_else(|e| {
                        error!("{}", e);
                    });
            }

            if !error.is_channel_closed() {
                error!("{}", error);
                let mut source = error.source();
                while let Some(e) = source {
                    error!("Caused by: {}", e);
                    source = e.source();
                }
            }
        }
        Ok(Ok(())) => {
            debug!("exit");
        }
    }
}
