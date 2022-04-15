#[macro_use]
extern crate dotenv_codegen;

extern crate chrono;
extern crate redis;

use std::thread;
use std::time::{Duration, Instant};

use dotenv::dotenv;
use serenity::async_trait;
use serenity::client::{Client, Context};
use serenity::framework::standard::{
    macros::{command, group},
    CommandResult, StandardFramework,
};
use serenity::model::channel::Message;
use serenity::utils::{MessageBuilder, EmbedMessageBuilding};
use songbird::SerenityInit;

mod services;
use services::music::*;

#[group]
#[commands(ping, play, stop, author)]
struct General;

const TOKEN: &'static str = dotenv!("TOKEN");
static mut redis_client: Option<redis::Client> = None;

struct Handler;

#[async_trait]
impl serenity::client::EventHandler for Handler {}

#[tokio::main]
async fn main() {
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~"))
        .group(&GENERAL_GROUP);

    unsafe {
        // redis_client = Some(redis::Client::open("host").unwrap());
    }

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
    let package_lengh = 15;
    let mut latencies: Vec<u128> = Vec::new();
    let mut base_msg = msg.reply(&ctx, "Checando conectividade").await.unwrap();

    for i in 0..package_lengh {
        let start = Instant::now();
        base_msg
            .edit(&ctx, |m| {
                m.content(format!("Enviando pacotes... {}/{}", i + 1, package_lengh))
            })
            .await
            .unwrap();
        let elapsed = start.elapsed();

        latencies.push(elapsed.as_millis());

        thread::sleep(Duration::from_millis(1000));
    }

    let sum = latencies.iter().fold(0, |s, val| s + val);
    let final_latency = sum / latencies.len() as u128;

    base_msg
        .edit(&ctx, |m| m.content(format!("Ping {}ms", final_latency)))
        .await
        .unwrap();

    Ok(())
}

#[command]
async fn author(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(&ctx, "LordShadd")
        .await
        .unwrap();

    Ok(())
}
