use git2::Repository;
use log::*;
use nvim_rs::create::tokio as create;
use std::default::Default;
use std::env;
use std::error::Error;
use std::sync::Arc;
//use tokio::sync::Mutex; // use std::sync::Mutex instead???
use std::sync::Mutex;
mod git;
mod node;
mod nvim;
mod tree;

use git::{fetch_and_pull, git_clone};
use node::init_codex_repo;
use nvim::NeovimHandler;

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
    match Repository::open("./") {
        Ok(_repo) => {
            // pull latest from remote, merge any updates from remote to local
            fetch_and_pull();
        }
        Err(_) => {
            if let Ok(git_remote_url) = std::env::var("CODEX_GIT_REMOTE") {
                debug!("cloning {}", &git_remote_url);
                git_clone(&git_remote_url).unwrap();
                debug!("{} successfully cloned!", &git_remote_url);
            } else {
                init_codex_repo();
            }
        }
    };
    let tree = Arc::new(Mutex::new(match tree::Tree::build("./".to_string()) {
        Ok(tree) => {
            debug!("tree gud!");
            tree
        }
        Err(e) => {
            error!("tree ERROR! {:?}", e);
            panic!("tree Error - PANIC {:?}", e);
        }
    }));
    let handler = NeovimHandler { tree };
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
