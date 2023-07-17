use std::time::Duration;
use std::{sync::Arc, thread};

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
        prelude::{Channel, ChannelId, Member},
        user::User,
    },
    utils::{Color, MessageBuilder},
    FutureExt,
};
use songbird::input::Input;
use songbird::tracks::TrackHandle;
use songbird::{EventHandler, Songbird, TrackEvent};

mod handler;
pub mod player;
pub mod playlist;
pub mod query;

use player::{PlayerError, PlayerStatus};

use self::player::{format_duration, MediaInfo};

#[group]
#[commands(play, pause, unpause, trackinfo, volume, skip, stop, playlist)]
struct Music;

macro_rules! send_embed {
    ($ctx:expr, $channel_id:expr, $embed:expr) => {{
        let mut msg = CreateMessage::default();

        msg.embed($embed);

        let map = hashmap_to_json_map(msg.0);

        $ctx.http
            .send_message($channel_id.0, &Value::Object(map))
            .await;
    }};
}

pub async fn send_media_message(
    ctx: &Context,
    member: &Member,
    channel_id: ChannelId,
    track_handle: Arc<TrackHandle>,
) {
    let metadata = track_handle.metadata();

    let thumb = metadata.thumbnail.clone().unwrap_or("???".to_string());
    let channel = metadata.channel.clone().unwrap_or("???".to_string());
    let title = metadata.title.clone().unwrap_or("???".to_string());
    let duration = format_duration(metadata.duration.clone());
    let url = metadata.clone().source_url.unwrap_or("???".to_string());

    let raw_date = metadata.clone().date.unwrap();

    let date = unsafe {
        format!(
            "{}/{}/{}",
            raw_date.get_unchecked(6..8),
            raw_date.get_unchecked(4..6),
            raw_date.get_unchecked(0..4),
        )
    };

    let mut msg = CreateMessage::default();

    msg.embed(|e| {
        e.image(thumb)
            .color(0xc3e2e1)
            .author(|a| {
                a.name(member.user.name.clone())
                    .icon_url(member.user.avatar_url().unwrap())
            })
            .thumbnail(
                "https://cdn.icon-icons.com/icons2/1429/PNG/512/icon-robots-16_98547.png"
                    .to_string(),
            )
            .description("Rádio do Bender.")
            .field("", "", false)
            .field("", "", false)
            .field("Titulo", title, true)
            .field("Canal", channel, true)
            .field("", "", false)
            .field("Duração", duration, true)
            .field("Data", date, true)
            .field("", "", false)
        // .field("URL", url, false)
    });

    let map = hashmap_to_json_map(msg.0);

    ctx.http
        .send_message(channel_id.0, &Value::Object(map))
        .await;

    send_embed!(ctx, channel_id, |e| {
        e.field("DALILA 1", "ABC", false)
            .field("DALILA 2", "ABC", false)
    });
}

#[command]
#[only_in(guilds)]
pub async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let member = &msg.member(&ctx.http).await.unwrap();

    let status = player::add(
        ctx.clone(),
        args.message().to_string(),
        msg.guild_id.unwrap(),
        msg.channel_id,
        member.clone(),
    )
    .await;

    match status {
        Ok(PlayerStatus::Queued) => {
            msg.reply(&ctx.http, "A sua música foi adicionada na playlist.")
                .await
                .unwrap();
        }
        Ok(PlayerStatus::Playing(media_info)) => {
            send_media_message(
                &ctx,
                &msg.member(&ctx.http).await.unwrap(),
                msg.channel_id,
                media_info.to_owned(),
            )
            .await;
        }
        Err(PlayerError::MusicNotFound) => {
            msg.reply(&ctx.http, "Musica não encontrada.")
                .await
                .unwrap();
        }
        Err(PlayerError::UserOffVoiceChannel) => {
            msg.reply(&ctx.http, "Você precisa estar em um canal de voz.")
                .await
                .unwrap();
        }
        _ => {}
    }

    return Ok(());
}

#[command]
#[only_in(guilds)]
pub async fn pause(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    player::pause(ctx, msg.guild_id.unwrap()).await;

    return Ok(());
}

#[command]
#[only_in(guilds)]
pub async fn unpause(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    player::unpause(ctx, msg.guild_id.unwrap()).await;

    return Ok(());
}

#[command]
#[only_in(guilds)]
pub async fn trackinfo(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let info = player::trackinfo(ctx, msg.guild_id.unwrap()).await;

    msg.reply(&ctx.http, info).await.unwrap();

    return Ok(());
}

#[command]
#[only_in(guilds)]
pub async fn volume(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let new_volume = args.parse::<f32>().unwrap() / 100.;

    let info = player::volume(ctx, msg.guild_id.unwrap(), new_volume).await;

    msg.reply(&ctx.http, info).await.unwrap();

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
