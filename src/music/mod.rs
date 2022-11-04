use std::sync::Arc;
use std::time::Duration;

use lazy_static::lazy_static;
use serde_json::Value;
use serenity::{
    async_trait,
    builder::{CreateMessage, EditMessage},
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    futures::lock::Mutex,
    http::CacheHttp,
    model::{
        channel::Message,
        id::GuildId,
        prelude::{Channel, ChannelId},
        user::User,
    },
    utils::{hashmap_to_json_map, Color, MessageBuilder},
    FutureExt,
};
use songbird::input::Input;
use songbird::{EventHandler, Songbird, TrackEvent};

mod handler;
pub mod player;
pub mod playlist;
pub mod query;

use player::{PlayerError, PlayerStatus};

use self::player::MediaInfo;

pub async fn send_media_message(
    ctx: &Context,
    author: &User,
    channel_id: u64,
    media_info: MediaInfo,
) {
    let mut msg = CreateMessage::default();

    msg.embed(|e| {
        e.image(media_info.thumb)
            .color(0xffffff)
            .author(|a| {
                a.name(author.name.clone())
                    .icon_url(author.avatar_url().unwrap())
            })
            .thumbnail(
                "https://www.wdentalstudio.com/wp-content/uploads/2021/08/headphones2.gif"
                    .to_string(),
            )
            .description("Radiolão do bender.")
            .field("Artista", media_info.artist, false)
            .field("Tocando", media_info.title, false)
            .field("Duração", media_info.duration, false)
            .field("URL", media_info.url, false)
    });

    let map = hashmap_to_json_map(msg.0);

    ctx.http.send_message(channel_id, &Value::Object(map)).await;
}

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

    if status.is_err() {
        match status.as_ref().unwrap_err() {
            PlayerError::MusicNotFound => {
                msg.reply(&ctx.http, "Musica não encontrada.")
                    .await
                    .unwrap();
            }
            PlayerError::UserOffVoiceChannel => {
                msg.reply(&ctx.http, "Você precisa estar em um canal de voz.")
                    .await
                    .unwrap();
            }
        }
    }

    if status.is_ok() {
        match status.unwrap() {
            PlayerStatus::Queued => {
                msg.reply(&ctx.http, "A sua música foi adicionada na playlist.")
                    .await
                    .unwrap();
            }
            PlayerStatus::Playing(media_info) => {
                send_media_message(&ctx, &msg.author, msg.channel_id.0, media_info).await;
            }
        }
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
pub async fn skip(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    msg.reply(&ctx.http, "Pulando para a próxima música.").await.unwrap();
    
    let playing_next = player::skip(&ctx, msg.guild_id.unwrap().0, msg.channel_id.0).await;
    
    if !playing_next {
        msg.reply(&ctx.http, "Não há mais nenhuma música na playlist.").await.unwrap();
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
pub async fn stop(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    Ok(())
}
