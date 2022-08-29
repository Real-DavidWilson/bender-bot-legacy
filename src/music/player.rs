use std::{sync::Arc, time::Duration};

use lazy_static::lazy_static;
use serde_json::Value;
use serenity::{
    async_trait,
    builder::EditMessage,
    futures::lock::Mutex,
    http::CacheHttp,
    model::{
        prelude::{Channel, Guild, GuildId},
        user::User,
    },
    prelude::Context,
    utils::hashmap_to_json_map,
};
use songbird::{input::Input, EventHandler, Songbird, TrackEvent};

use super::{
    handler::StopMusicHandle,
    playlist::{self, PlaylistError},
    query::query_video,
};

type PlayerResult<T> = Result<T, PlayerError>;

lazy_static! {
    pub static ref IS_PLAYING: Mutex<bool> = Mutex::new(false);
}

#[derive(Debug, Copy, Clone)]
pub enum PlayerStatus {
    Playing,
    Queued,
}

#[derive(Debug, Copy, Clone)]
pub enum PlayerError {
    UserOffVoiceChannel,
    MusicNotFound,
}

fn format_duration(duration: Duration) -> String {
    let naive_duration = chrono::naive::NaiveTime::from_hms(
        (duration.as_secs() as u32 / 60) / 60,
        (duration.as_secs() as u32 / 60) % 60,
        duration.as_secs() as u32 % 60,
    );

    format!("{}", naive_duration.format("%M:%S").to_string())
}

pub async fn queue(
    ctx: Context,
    uri: String,
    guild: Guild,
    channel: Channel,
    author: User,
) -> PlayerResult<PlayerStatus> {
    let source = query_video(uri.clone()).await;

    if source.is_err() {
        return Err(PlayerError::MusicNotFound);
    }

    let source = source.unwrap();

    if *IS_PLAYING.lock().await {
        playlist::insert(playlist::PlaylistItem {
            ctx: ctx.clone(),
            uri: uri.clone(),
            guild,
            channel,
            author,
            source,
        })
        .await
        .unwrap();

        return Ok(PlayerStatus::Queued);
    }

    play(&ctx, source, guild, channel, author).await?;

    let mut is_playing = IS_PLAYING.lock().await;
    *is_playing = true;

    Ok(PlayerStatus::Playing)
}

pub async fn play(
    ctx: &Context,
    source: Input,
    guild: Guild,
    channel: Channel,
    author: User,
) -> PlayerResult<()> {
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states
        .get(&author.id)
        .and_then(|voice_state| voice_state.channel_id);

    if channel_id.is_none() {
        return Err(PlayerError::UserOffVoiceChannel);
    }

    let connect_to = channel_id.unwrap();

    let manager = songbird::get(&ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
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
    let video_duration = metadata.duration.unwrap();
    let url = metadata.source_url.unwrap();

    let duration = format_duration(video_duration);

    handler.stop();
    let track_handle = handler.play_only_source(source);
    handler.deafen(true).await.unwrap();

    track_handle
        .add_event(
            songbird::Event::Track(TrackEvent::End),
            StopMusicHandle {
                ctx: ctx.clone(),
                channel_id: channel_id.unwrap().0,
                manager: manager.clone(),
                guild_id,
            },
        )
        .unwrap();

    Ok(())
}

pub async fn stop() {}
