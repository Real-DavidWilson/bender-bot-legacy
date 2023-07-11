use std::{collections::HashMap, sync::Arc, time::Duration};

use chrono::{naive, Timelike};
use lazy_static::lazy_static;
use serde_json::Value;
use serenity::{
    async_trait,
    builder::{CreateMessage, EditMessage},
    futures::lock::Mutex,
    http::CacheHttp,
    model::{
        prelude::{Channel, Embed, Guild, GuildId},
        user::User,
    },
    prelude::{ClientError, Context},
    utils::{CustomMessage, MessageBuilder},
};
use songbird::{input::Input, EventHandler, Songbird, TrackEvent};

use super::{
    handler::{self, StopMusicHandle},
    playlist::{self, PlaylistError},
    query::query_video,
    send_media_message,
};

type PlayerResult<T> = Result<T, PlayerError>;

lazy_static! {
    pub static ref IS_PLAYING: Mutex<HashMap<u64, bool>> = Mutex::new(HashMap::new());
}

#[derive(Debug, Clone)]
pub enum PlayerStatus {
    Playing(MediaInfo),
    Queued,
}

#[derive(Debug, Copy, Clone)]
pub enum PlayerError {
    UserOffVoiceChannel,
    MusicNotFound,
}

#[derive(Debug, Clone)]
pub struct MediaInfo {
    pub title: String,
    pub thumb: String,
    pub artist: String,
    pub video_duration: Option<Duration>,
    pub url: String,
    pub duration: String,
}

pub fn format_duration(duration: Option<Duration>) -> String {
    if duration.is_none() {
        return String::from("Ao Vivo")
    }

    let duration = duration.unwrap();

    let naive_duration = chrono::naive::NaiveTime::from_hms(
        (duration.as_secs() as u32 / 60) / 60,
        (duration.as_secs() as u32 / 60) % 60,
        duration.as_secs() as u32 % 60,
    );

    if naive_duration.hour() > 0 {
        return format!("{}", naive_duration.format("%H:%M:%S").to_string());
    }

    format!("{}", naive_duration.format("%M:%S").to_string())
}

pub async fn queue<'a>(
    ctx: Context,
    uri: String,
    guild: Guild,
    channel: Channel,
    author: User,
) -> PlayerResult<PlayerStatus> {
    let source = query_video(uri.clone()).await;
    let guild_id = guild.id.0;

    if source.is_err() {
        return Err(PlayerError::MusicNotFound);
    }

    let source = source.unwrap();

    if *IS_PLAYING.lock().await.get(&guild.id.0).unwrap_or(&false) {
        playlist::insert(
            guild.id.0,
            playlist::PlaylistItem {
                ctx: ctx.clone(),
                uri: uri.clone(),
                guild,
                channel,
                author,
                source,
            },
        )
        .await
        .unwrap();

        return Ok(PlayerStatus::Queued);
    }

    let media_info = play(&ctx, source, guild, channel, author).await?;

    IS_PLAYING.lock().await.insert(guild_id, true);

    Ok(PlayerStatus::Playing(media_info))
}

pub async fn play(
    ctx: &Context,
    source: Input,
    guild: Guild,
    channel: Channel,
    author: User,
) -> PlayerResult<MediaInfo> {
    let guild_id = guild.id;

    let voice_channel_id = guild
        .voice_states
        .get(&author.id)
        .and_then(|voice_state| voice_state.channel_id);

    if voice_channel_id.is_none() {
        return Err(PlayerError::UserOffVoiceChannel);
    }

    let connect_to = voice_channel_id.unwrap();

    let manager = songbird::get(&ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.")
        .clone();

    let _ = manager.join(guild_id, connect_to).await;

    let handler_lock = manager.get(guild_id);

    if handler_lock.is_none() {
        return Err(PlayerError::UserOffVoiceChannel);
    }

    let mut handler = handler_lock.as_ref().unwrap().lock().await;

    let metadata = source.metadata.clone();

    let title = metadata.title.unwrap();
    let thumb = metadata.thumbnail.unwrap();
    let artist = metadata.artist.unwrap();
    let video_duration = metadata.duration;
    let url = metadata.source_url.unwrap();

    let duration = format_duration(metadata.duration);

    let has_connection = handler.current_connection().is_some();

    if has_connection {
        handler.stop();
    }

    let track_handle = handler.play_source(source);
    handler.deafen(true).await.unwrap();

    track_handle
        .add_event(
            songbird::Event::Track(TrackEvent::End),
            StopMusicHandle {
                ctx: ctx.clone(),
                channel_id: channel.id().0,
                guild_id: guild_id.0,
            },
        )
        .unwrap();

    Ok(MediaInfo {
        title,
        thumb,
        artist,
        video_duration,
        url,
        duration,
    })
}

pub async fn play_next(ctx: &Context, guild_id: u64, channel_id: u64) -> bool {
    let manager = songbird::get(&ctx).await.unwrap();
    let next_music = playlist::next(guild_id).await;

    if next_music.is_none() {
        IS_PLAYING.lock().await.insert(guild_id, false);

        let handler_lock = manager.get(guild_id);

        if handler_lock.is_none() {
            return false;
        }

        let mut handler = handler_lock.as_ref().unwrap().lock().await;

        let on_channel = handler.current_channel().is_some();
        let has_connection = handler.current_connection().is_some();

        if has_connection {
            handler.stop();
        }

        if on_channel {
            handler.leave().await.unwrap();
        }

        return false;
    }

    let item = next_music.unwrap();

    let media_info = play(
        &item.ctx,
        item.source,
        item.guild,
        item.channel.clone(),
        item.author.clone(),
    )
    .await
    .unwrap();

    send_media_message(&ctx, &item.author, item.channel.id().0, media_info).await;

    return true;
}

pub async fn skip(ctx: &Context, guild_id: u64, channel_id: u64) -> bool {
    play_next(&ctx, guild_id, channel_id).await
}

pub async fn stop(ctx: &Context, guild_id: u64) {
    let manager = songbird::get(&ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = manager.get(guild_id);

    if handler_lock.is_none() {
        return;
    }

    let mut handler = handler_lock.as_ref().unwrap().lock().await;

    handler.stop();
    handler.leave().await.unwrap();

    playlist::reset(guild_id).await;
    IS_PLAYING.lock().await.insert(guild_id, false);
}
