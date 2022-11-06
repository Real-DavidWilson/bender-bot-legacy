#![allow(unused)]
#[macro_use]
extern crate dotenv_codegen;

extern crate chrono;
extern crate redis;

use std::ops::Sub;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serenity::async_trait;
use serenity::client::{Client, Context};
use serenity::framework::standard::{
    macros::{command, group},
    CommandResult, StandardFramework,
};
use serenity::model::channel::Message;
use serenity::model::prelude::ReactionType;
use songbird::SerenityInit;

mod chat;
mod music;
use chat::*;
use music::*;
use tracing_subscriber::fmt::format;

#[group]
#[commands(ping, play, skip, stop, author, clear)]
struct General;

const TOKEN: &'static str = dotenv!("TOKEN");
// static mut REDIS_CONN: Option<redis::Connection> = None;

struct Handler;

#[async_trait]
impl serenity::client::EventHandler for Handler {}

#[tokio::main]
async fn main() {
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~"))
        .group(&GENERAL_GROUP);

    let mut client = Client::builder(TOKEN)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let latency = msg.timestamp.timestamp_millis();

    msg.reply(&ctx.http, format!("Ping {}ms", now - latency as u128)).await.unwrap();

    Ok(())
}

#[command]
async fn author(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(&ctx, "David Wilson - davidwilsonbr2019@gmail.com")
        .await
        .unwrap();

    Ok(())
}
