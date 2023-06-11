use std::ops::Sub;

use serenity::{
    framework::standard::{
        macros::{command, group},
        CommandResult,
    },
    model::prelude::Message,
    prelude::Context,
};

use chrono::Local;

#[group]
#[commands(ping)]
pub struct Network;

#[command]
pub async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    let msg_timestamp = msg.timestamp.naive_utc().timestamp_millis();
    let now = Local::now().naive_utc().timestamp_millis();

    let latency = now.sub(msg_timestamp);

    msg.reply(&ctx.http, format!("Ping {}ms", latency))
        .await
        .unwrap();

    Ok(())
}
