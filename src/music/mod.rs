use std::sync::Arc;
use std::time::Duration;

use lazy_static::lazy_static;
use serde_json::Value;
use serenity::{
    async_trait,
    builder::{CreateMessage, EditMessage},
    client::Context,
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    futures::lock::Mutex,
    http::CacheHttp,
    json::hashmap_to_json_map,
    model::{
        channel::Message,
        id::GuildId,
        prelude::{Channel, ChannelId},
        user::User,
    },
    utils::{Color, MessageBuilder},
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

#[group]
#[commands(play, skip, stop, playlist)]
struct Music;

pub async fn send_media_message(
    ctx: &Context,
    author: &User,
    channel_id: u64,
    media_info: MediaInfo,
) {
    let mut msg = CreateMessage::default();

    msg.embed(|e| {
        e.image(media_info.thumb)
            .color(0xc3e2e1)
            .author(|a| {
                a.name(author.name.clone())
                    .icon_url(author.avatar_url().unwrap())
            })
            .thumbnail(
                "https://cdn.icon-icons.com/icons2/1429/PNG/512/icon-robots-16_98547.png"
                    .to_string(),
            )
            .description("Rádio do Bender.")
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
    let guild = msg.guild(&ctx.cache).unwrap();
    let channel = msg.channel(&ctx.http).await.unwrap();

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

        return Ok(());
    }

    match status.as_ref().unwrap() {
        PlayerStatus::Queued => {
            msg.reply(&ctx.http, "A sua música foi adicionada na playlist.")
                .await
                .unwrap();
        }
        PlayerStatus::Playing(media_info) => {
            send_media_message(&ctx, &msg.author, msg.channel_id.0, media_info.to_owned()).await;
        }
    }

    return Ok(());
}

#[command]
#[only_in(guilds)]
pub async fn playlist(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let page = args.parse::<usize>().unwrap_or(1);
    let guild_id = msg.guild(&ctx.cache).unwrap().id.0;
    let channel_id = msg.channel_id.0;

    let playlist_info = playlist::info(guild_id, page, 3).await;

    if playlist_info.is_none() {
        return Ok(());
    }

    let playlist_info = playlist_info.unwrap();

    let mut items_str = String::new();

    for (i, item_info) in playlist_info.items.iter().enumerate() {
        if i > 0 {
            items_str.push_str("\n");
        }

        let index = item_info.index + 1;
        let title = item_info.media_info.title.clone();
        let artist = item_info.media_info.artist.clone();

        items_str.push_str(format!("`{index}° - {title} | {artist}`").as_str());
    }

    let mut msg_build = CreateMessage::default();

    msg_build.embed(|e| {
        e.author(|a| {
            a.name(msg.author.name.clone())
                .icon_url(msg.author.avatar_url().unwrap())
        })
        .description("Playlist")
        .field("Musgas", playlist_info.total_tracks, true)
        .field("Paginas", playlist_info.total_pages, true)
        .field("Musga de homi", items_str, false)
        .footer(|f| f.text("Bora beber pinga"))
    });

    let map = hashmap_to_json_map(msg_build.0);

    ctx.http.send_message(channel_id, &Value::Object(map)).await;

    return Ok(());
}

#[command]
#[only_in(guilds)]
pub async fn skip(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let playing_next = player::skip(&ctx, msg.guild_id.unwrap().0, msg.channel_id.0).await;

    if !playing_next {
        msg.reply(&ctx.http, "Não há mais nenhuma música na playlist.")
            .await
            .unwrap();
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
pub async fn stop(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap().0;

    player::stop(ctx, guild_id).await;

    Ok(())
}
