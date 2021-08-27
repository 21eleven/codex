use async_trait::async_trait;
use log::*;
use nvim_rs::{compat::tokio::Compat, create::tokio as create, Handler, Neovim};
use rmpv::Value;
use std::default::Default;
use std::error::Error;
use tokio::io::Stdout;
use tokio::time;

#[derive(Clone)]
struct NeovimHandler {}

#[async_trait]
impl Handler for NeovimHandler {
    type Writer = Compat<Stdout>;

    // async fn handle_notify(&self, name: String, _args: Vec<Value>, neovim: Neovim<Handler::Writer>) {
    // async fn handle_notify(&self, name: String, _args: Vec<Value>, neovim: Neovim<<Type as Handler>::Writer>) {
    async fn handle_notify(&self, name: String, _args: Vec<Value>, neovim: Neovim<Compat<Stdout>>) {
        match name.as_ref() {
            "start" => {
                neovim.command("lua print(\"hello plugin started\")")
                    .await
                    .unwrap();
            }
            "ping" => {
                let args_str = format!("{:?}", _args);
                let s = format!("lau print(\"hello pong {}\")", args_str.replace('"', "\\\""));
                neovim.command(s.as_str()).await.unwrap();
            }
            _ => {}
        }
    }
    async fn handle_request(&self, _name: String, _args: Vec<Value>, _neovim: Neovim<Compat<Stdout>>) -> Result<Value, Value> {
        Ok(Value::Nil)
    }
}


fn main() {
    println!("Hello, world!");
}
