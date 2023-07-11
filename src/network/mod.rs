use std::ops::Sub;

use serenity::{
    framework::standard::{
        macros::{command, group},
        CommandResult,
    },
    model::{prelude::Message, Timestamp},
    prelude::Context,
};

use chrono::Local;

#[group]
#[commands(ping)]
pub struct Network;

#[command]
pub async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    let mut now = Timestamp::now().naive_utc().timestamp_millis();
    let msg_timestamp = msg.timestamp.naive_utc().timestamp_millis();

    let latency = msg_timestamp - now;

    msg.reply(&ctx.http, format!("Ping {latency}ms"))
        .await
        .unwrap();

    Ok(())
}
