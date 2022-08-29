use std::sync::Arc;
use std::time::Duration;

use lazy_static::lazy_static;
use serde_json::Value;
use serenity::{
    async_trait,
    builder::EditMessage,
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    futures::lock::Mutex,
    http::CacheHttp,
    model::{channel::Message, id::GuildId, prelude::ChannelId},
    utils::{hashmap_to_json_map, Color, MessageBuilder},
    FutureExt,
};
use songbird::input::Input;
use songbird::{EventHandler, Songbird, TrackEvent};

pub mod player;
pub mod playlist;
pub mod query;
mod handler;

use player::{PlayerError, PlayerStatus};

#[command]
#[only_in(guilds)]
pub async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    println!("Command executed");

    let guild = msg.guild(&ctx.cache).await.unwrap();
    let channel = msg.channel(&ctx.cache).await.unwrap();

    let status = player::queue(
        ctx.clone(),
        args.message().to_string(),
        guild,
        channel,
        msg.author.clone(),
    )
    .await;

    let mut message = MessageBuilder::new();
    message.mention(&msg.author);

    if status.is_err() {
        match status.unwrap_err() {
            PlayerError::MusicNotFound => {
                message.push("Musica não encontrada!");
            }
            PlayerError::UserOffVoiceChannel => {
                println!("OFF VOICE CHANNEL");
                message.push("Você precisa estar em um canal de voz!");
            }
        }
    }

    if status.is_ok() {
        match status.unwrap() {
            PlayerStatus::Queued => {
                message.push("Adicionado à fila!");
            }
            PlayerStatus::Playing => {
                message.push("Tocando agora!");
            }
        }
    }

    msg.channel_id.say(&ctx.http, message).await.unwrap();

    Ok(())
}

#[command]
#[only_in(guilds)]
pub async fn stop(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    Ok(())
}
