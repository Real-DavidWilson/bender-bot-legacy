#![allow(unused)]
#[macro_use]
extern crate dotenv_codegen;

extern crate chrono;
extern crate redis;

use std::ops::{Sub, SubAssign};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chrono::TimeZone;
use serenity::async_trait;
use serenity::client::{Client, Context};
use serenity::framework::standard::macros::hook;
use serenity::framework::standard::{
    macros::{command, group},
    CommandResult, StandardFramework,
};
use serenity::model::prelude::Message;
use serenity::prelude::{EventHandler, GatewayIntents};
use songbird::packet::pnet::types::u1;
use songbird::SerenityInit;
use tracing_subscriber::fmt::format;

use chat::*;
use music::*;
use network::*;

mod chat;
mod music;
mod network;

#[group]
#[commands(ping)]
struct General;

const TOKEN: &'static str = dotenv!("TOKEN");

#[hook]
async fn unknown_command(_ctx: &Context, _msg: &Message, unknown_command_name: &str) {
    println!("Could not find command named '{}'", unknown_command_name);
}

#[tokio::main]
async fn main() {
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("."))
        .unrecognised_command(unknown_command)
        .group(&GENERAL_GROUP)
        .group(&MUSIC_GROUP)
        .group(&NETWORK_GROUP);

    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::GUILDS
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILD_MESSAGES;

    let mut client = Client::builder(TOKEN, intents)
        .register_songbird()
        .framework(framework)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}
