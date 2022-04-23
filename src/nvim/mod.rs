use async_trait::async_trait;
use log::*;
use nvim_rs::{compat::tokio::Compat, Handler, Neovim};
use std::path::PathBuf;
use std::sync::Arc;

use crate::tree;
use crate::tree::next_sibling_id;
//use tokio::sync::Mutex; // use std::sync::Mutex instead???
use crate::git::diff::{
    diff_w_last_commit, diff_w_last_commit_report, diff_w_main, diff_w_main_report,
    repo_is_modified,
};
use crate::git::{
    commit_all, fetch_and_pull, get_last_commit_of_branch, handle_git_branching,
    push_to_git_remote, repo, stage_all,
};
use crate::node::power_of_ten;
use rmpv::Value;
use std::env;
use std::sync::Mutex;
use tokio::io::Stdout;
use tokio::time;

pub trait Telescoped {
    fn entry(&self) -> Value;
}

fn telescope_nodes(tree: &tree::Tree) -> Value {
    Value::Array(
        tree.nodes
            .values()
            // impl DeRef to Value?
            .map(|n| n.entry())
            .collect(),
    )
}

fn telescope_child_nodes(id: &str, tree: &tree::Tree) -> Value {
    // impl use lua data type?
    // impl nvim data trait?
    match tree.nodes.get(id) {
        Some(node) => Value::Array(node.children.iter().map(|key| key.entry()).collect()),
        None => Value::Nil,
    }
}

// make method on tree
fn node_parent(id: &str, tree: &tree::Tree) -> Option<String> {
    match tree.nodes.get(id) {
        Some(node) => node.parent.clone(),
        None => None,
    }
}

#[derive(Clone)]
pub struct NeovimHandler {
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
        // could make a sync fn that handles RPC commit_all
        // and also handles Err results
        match name.as_ref() {
            "start" => {
                log::debug!("starting CODEX!");
                if let Some(dir) = env::current_dir().unwrap().to_str() {
                    neovim.command(&format!("cd {}", dir)).await.unwrap()
                }
                debug!("pwd: {:?}", std::env::current_dir().unwrap());
                on_start(neovim.clone()).await;
                fetch_and_pull().unwrap();
                handle_git_branching().unwrap();
                let today = self.tree.lock().unwrap().today_node();
                neovim.command(&format!("e {}/_.md", today)).await.unwrap();
                stage_all().unwrap();

                // let today = tree.today_node();
                debug!("git remote url {:?}", std::env::var("CODEX_GIT_REMOTE"));
            }
            "has_diff" => {
                debug!("has diffs? {}", repo_is_modified().unwrap());
            }
            "diff" => {
                let added = diff_w_main().unwrap();
                debug!("words added (vs main): {}", added);
                neovim
                    .command(&format!("lua print('words: {}')", added))
                    .await
                    .unwrap();
            }
            "diff_last" => {
                let added = diff_w_last_commit().unwrap();
                debug!("words added (vs prev commit): {}", added);
            }
            "diff_report" => {
                let report = diff_w_main_report().unwrap();
                debug!("Diff Report (vs main): {}", report);
            }
            "diff_last_report" => {
                let report = diff_w_last_commit_report().unwrap();
                debug!("Diff Report (vs last commit): {}", report);
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
            "push" => {
                debug!("push status: {:?}", push_to_git_remote().await);
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
                // weird error i don't understack if I uncomment this...
                // let tree = &mut *self.tree.lock().unwrap();
                let new_node_key = self.tree.lock().unwrap().node_creation(_args).unwrap();
                neovim
                    .command(&format!("e {}/_.md", new_node_key))
                    .await
                    .unwrap();
                stage_all().unwrap();
            }
            "node" => {
                let args: Vec<Option<&str>> = _args.iter().map(|arg| arg.as_str()).collect();
                let tree = &*self.tree.lock().unwrap();
                if let [Some(node_ref)] = args.as_slice() {
                    if let Some(node) = tree.nodes.get(&node_ref.to_string()) {
                        debug!("{:?}: {}", node_ref, node);
                    } else {
                        debug!("{:?} not found", node_ref);
                    }
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
                        let id = next_sibling_id(&PathBuf::from(dir.to_string()));
                        debug!("SIB ID: {}", id);
                    }
                    _ => {}
                }
            }
            // "stop" => {
            //     push_to_git_remote().unwrap();
            //     info!("local pushed to remote");
            //
            //     tokio::spawn(async move {
            //         let mut interval = time::interval(time::Duration::from_secs(3));
            //         interval.tick().await;
            //         debug!("woke up, closing");
            //     });
            // }
            _ => {}
        }
    }
    async fn handle_request(
        &self,
        name: String,
        _args: Vec<Value>,
        neovim: Neovim<Compat<Stdout>>,
    ) -> Result<Value, Value> {
        debug!("in request handler");
        match name.as_str() {
            "stop" => {
                if repo_is_modified().unwrap() {
                    push_to_git_remote().await.unwrap();
                    info!("local pushed to remote");
                }
                let mut interval = time::interval(time::Duration::from_secs(3));
                interval.tick().await;
                debug!("woke up, closing");
                Ok(Value::Nil)
            }
            "nodes" => Ok(telescope_nodes(&*self.tree.lock().unwrap())),
            "children" => {
                debug!("{:?}", _args);
                let args: Vec<&str> = _args.iter().map(|arg| arg.as_str().unwrap()).collect();
                Ok(telescope_child_nodes(args[0], &*self.tree.lock().unwrap()))
            }
            "parent" => {
                debug!("{:?}", _args);
                let args: Vec<&str> = _args.iter().map(|arg| arg.as_str().unwrap()).collect();
                if let Some(parent) = node_parent(args[0], &*self.tree.lock().unwrap()) {
                    debug!("found parent: {}", parent);
                    Ok(Value::String(parent.into()))
                } else {
                    Ok(Value::Nil)
                }
            }
            "latest-journal" => {
                let page = self.tree.lock().unwrap().latest_journal();
                debug!("{page}");
                Ok(Value::String(page.into()))
            }
            "prev-sibling" => {
                let args: Vec<&str> = _args.iter().map(|arg| arg.as_str().unwrap()).collect();
                Ok(Value::String(self.tree.lock().unwrap().next_sibling(args[0], true).into()))
            }
            "next-sibling" => {
                let args: Vec<&str> = _args.iter().map(|arg| arg.as_str().unwrap()).collect();
                Ok(Value::String(self.tree.lock().unwrap().next_sibling(args[0], false).into()))
            }
            _ => Ok(Value::Nil),
        }
    }
}
