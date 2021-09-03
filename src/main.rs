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
use tokio::sync::Mutex; // use std::sync::Mutex instead???
use tokio::time;
mod node;
use node::{lay_foundation, Node, Page};

#[derive(Clone)]
struct NeovimHandler {
    // repo: Arc<Mutex<Option<Repository>>>,
    repo: Arc<Mutex<Repository>>,
}

async fn on_start(nvim: Neovim<Compat<Stdout>>) {
    let yyyymmdd = Local::now().format("%Y%m%d");
    // for (k, v) in env::vars() {
    //     log::debug!("::env {}: {}", k, v);
    // }
    // let tnode: Page = Node::new("test".to_string());
    // debug!("{:?}", tnode);
    // let nodestring = toml::to_string_pretty(&tnode).unwrap();
    // debug!("{:?}", nodestring);
    // let tnode2: Page = toml::from_str(&nodestring).unwrap();
    // debug!("{:?}", tnode2);

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
                log::debug!("{:?}", self.repo.lock().await.state());
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
            _ => Ok(Value::Nil) 

        }
        
    }
}

#[tokio::main]
async fn main() {
    let plugin_dir = if let Ok(dir) = std::env::var("CODEX_HOME") {
        dir
    } else {
        std::env::set_var("CODEX_HOME", ".");
        ".".to_string()
    };
    let config_file = format!("{}/codex-log.toml", plugin_dir);

    log_panics::init();
    if let Err(e) = log4rs::init_file(format!("{}/codex-log.toml", plugin_dir), Default::default())
    {
        eprintln!("Error configuring logging with {}: {:?}", config_file, e);
        return;
    }
    debug!("{:?}", env::current_dir().unwrap());
    // When opening the repo we could inspect the result and init the repo
    // and build the foundational nodes
    let repo = Arc::new(Mutex::new(match Repository::open("./data") {
        Ok(repo) => repo,
        Err(_) => {
            lay_foundation();
            Repository::init("./data").unwrap()
        }
    }));
    // let repo = Arc::new(Mutex::new(Repository::open("./data").unwrap()));
    // interval.tick().await;
    let handler = NeovimHandler { repo };
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
