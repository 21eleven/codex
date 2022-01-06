use async_trait::async_trait;
use git2::{DiffFormat, Repository};
use log::*;
use nvim_rs::{compat::tokio::Compat, Handler, Neovim};
use std::sync::Arc;

use crate::tree;
use crate::tree::next_sibling_id;
use chrono::Local;
//use tokio::sync::Mutex; // use std::sync::Mutex instead???
use crate::git::{
    commit_all, get_last_commit_of_branch,
    handle_git_branching, repo, stage_all, diff_w_last_commit, diff_w_main
};
use crate::node::power_of_ten;
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
                debug!("pwd: {:?}", std::env::current_dir().unwrap());
                log::debug!("{:?}", self.repo.lock().unwrap().state());
                let today = self.tree.lock().unwrap().today_node();
                handle_git_branching().unwrap();

                match env::current_dir().unwrap().to_str() {
                    Some(dir) => neovim.command(&format!("cd {}/codex", dir)).await.unwrap(),
                    None => {}
                }
                // let today = tree.today_node();
                neovim.command(&format!("e {}/_.md", today)).await.unwrap();
                on_start(neovim).await;
            }
            "diff" => {
                let added = diff_w_main().unwrap();
                debug!("words added (vs main): {}", added);
            }
            "diff_last" => {
                let added = diff_w_last_commit().unwrap();
                debug!("words added (vs prev commit): {}", added);
            }
            "stage" => {
                stage_all().unwrap();
                // commit_all(None).unwrap();
            }
            "commit" => {
                commit_all(None).unwrap();
            }
            "branch_commit" => {
                let repo = repo().unwrap();
                let branch_name = _args[0].as_str().unwrap();
                let commit = get_last_commit_of_branch(&repo, branch_name);
                debug!("{}: {:?}", branch_name, commit);
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
                        count += 1;
                    }
                });
            }
            "create" => {
                debug!("{:?}", _args);
                let tree = &mut *self.tree.lock().unwrap();
                tree.node_creation(_args);
            }
            "node" => {
                let args: Vec<Option<&str>> = _args.iter().map(|arg| arg.as_str()).collect();
                let tree = &*self.tree.lock().unwrap();
                match args.as_slice() {
                    &[Some(node_ref)] => {
                        debug!(
                            "{:?}: {}",
                            node_ref,
                            tree.nodes.get(&node_ref.to_string()).unwrap()
                        );
                    }
                    _ => {}
                }
            }
            "pow" => {
                debug!("{:?}", _args);
                let arg: Vec<u64> = _args
                    .iter()
                    .map(|arg| arg.as_str().unwrap_or(""))
                    .flat_map(|e| e.parse::<u64>())
                    .collect();
                debug!("{:?}", arg);
                match arg.as_slice() {
                    &[n] => debug!("{}: pow? {:?}", n, power_of_ten(n)),
                    _ => debug!("supply a single u64 for pow"),
                }
            }
            "test_sib" => {
                let args: Vec<Option<&str>> = _args.iter().map(|arg| arg.as_str()).collect();
                match args.as_slice() {
                    &[Some(dir)] => {
                        let id = next_sibling_id(&dir.to_string());
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
            "nodes" => {
                let tree = &*self.tree.lock().unwrap();
                // let mut nodes: Vec<&str> = tree
                let nodes: Vec<String> = tree.nodes.keys().map(|id| id.to_string()).collect();
                // since I am sorting here maybe I should
                // switch from HashMap to BTreeMap
                // nodes.sort_unstable();

                Ok(Value::Array(
                    nodes.into_iter().map(|s| Value::String(s.into())).collect(),
                ))
            }
            _ => Ok(Value::Nil),
        }
    }
}
