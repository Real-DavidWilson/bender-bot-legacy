use std::{
    collections::HashMap,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    time::Duration,
};

use chrono::{naive, Local, Timelike};
use lazy_static::lazy_static;
use serde_json::Value;
use serenity::{
    async_trait,
    builder::{CreateMessage, EditMessage},
    http::CacheHttp,
    model::{
        prelude::{Channel, ChannelId, Embed, Guild, GuildId, Member},
        user::User,
    },
    prelude::{ClientError, Context},
    utils::{CustomMessage, MessageBuilder},
};
use songbird::{
    create_player,
    input::Input,
    tracks::{PlayMode, Track, TrackCommand, TrackHandle},
    EventHandler, Songbird, TrackEvent,
};
use tokio::sync::Mutex;

use super::{
    handler::{self, StopMusicHandle},
    playlist::{self, PlaylistError},
    query::query_video,
    send_media_message,
};

type PlayerResult<T> = Result<T, PlayerError>;

lazy_static! {
    pub static ref CURRENT_TRACKS: Mutex<HashMap<u64, Arc<TrackHandle>>> =
        Mutex::new(HashMap::new());
}

#[derive(Debug, Clone)]
pub enum PlayerStatus {
    Playing(Arc<TrackHandle>),
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
        return String::from("Ao Vivo");
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

pub async fn add(
    ctx: Context,
    uri: String,
    guild_id: GuildId,
    channel_id: ChannelId,
    member: Member,
) -> PlayerResult<PlayerStatus> {
    let source = query_video(uri.clone()).await;

    if source.is_err() {
        return Err(PlayerError::MusicNotFound);
    }

    let source = source.unwrap();

    let current_track = CURRENT_TRACKS.lock().await.get(&guild_id.0);
    let mut can_play = true;

    match CURRENT_TRACKS.lock().await.get(&guild_id.0) {
        Some(track_handle) => {
            can_play = match track_handle.get_info().await.unwrap().playing {
                PlayMode::End | PlayMode::Stop => true,
                _ => false,
            };
        }
        _ => {}
    };

    if !can_play {
        playlist::insert(
            guild_id.0,
            playlist::PlaylistItem {
                ctx: ctx.clone(),
                uri: uri.clone(),
                guild_id,
                channel_id,
                member,
                source,
            },
        )
        .await
        .unwrap();

        return Ok(PlayerStatus::Queued);
    }

    let track_handle = play(&ctx, source, guild_id, channel_id, member).await?;

    let track_handler = Arc::new(track_handle);

    CURRENT_TRACKS
        .lock()
        .await
        .insert(guild_id.0, track_handler.clone());

    Ok(PlayerStatus::Playing(track_handler.clone()))
}

pub async fn play(
    ctx: &Context,
    source: Input,
    guild_id: GuildId,
    channel_id: ChannelId,
    member: Member,
) -> PlayerResult<TrackHandle> {
    let guild = guild_id.to_guild_cached(&ctx.cache).unwrap();

    let voice_channel_id = guild
        .voice_states
        .get(&member.user.id)
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

    let mut handler = match manager.get(guild_id) {
        Some(handler) => handler.lock_owned().await,
        _ => return Err(PlayerError::UserOffVoiceChannel),
    };

    let (track, track_handle) = create_player(source);

    handler.play_only(track);
    handler.deafen(true).await.unwrap();

    track_handle
        .add_event(
            songbird::Event::Track(TrackEvent::End),
            StopMusicHandle {
                ctx: ctx.clone(),
                channel_id,
                guild_id,
            },
        )
        .unwrap();

    Ok(track_handle)
}

pub async fn next(ctx: &Context, guild_id: u64, channel_id: u64) -> bool {
    let manager = songbird::get(&ctx).await.unwrap();
    let next_item = playlist::next(guild_id).await;

    if next_item.is_none() {
        CURRENT_TRACKS.lock().await.remove(&guild_id);

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

    let item = next_item.unwrap();

    let track_handle = play(
        &item.ctx,
        item.source,
        item.guild_id,
        item.channel_id,
        item.member.clone(),
    )
    .await
    .unwrap();

    let track_handler = Arc::new(track_handle);

    CURRENT_TRACKS
        .lock()
        .await
        .insert(guild_id, track_handler.clone());

    send_media_message(&ctx, &item.member, item.channel_id, track_handler.clone()).await;

    return true;
}

pub async fn pause(ctx: &Context, guild_id: GuildId) {
    let current_track = CURRENT_TRACKS.lock().await;

    let track_handle = match current_track.get(&guild_id.0) {
        Some(track_handle) => track_handle,
        _ => return,
    };

    track_handle.pause();
}

pub async fn unpause(ctx: &Context, guild_id: GuildId) {
    let current_track = CURRENT_TRACKS.lock().await;

    let track_handle = match current_track.get(&guild_id.0) {
        Some(track_handle) => track_handle,
        _ => return,
    };

    track_handle.play();
}

pub async fn trackinfo(ctx: &Context, guild_id: GuildId) -> String {
    let current_track = CURRENT_TRACKS.lock().await;

    let track_handle = match current_track.get(&guild_id.0) {
        Some(track_handle) => track_handle,
        _ => return "Não há nenhuma música tocando.".to_string(),
    };

    let info = track_handle.get_info().await.unwrap();

    let position = Some(info.position.clone());
    let duration = track_handle.metadata().duration.clone();
    let volume = info.volume.clone();

    format!(
        "Duração: {}/{}\nVolume: {}/100",
        format_duration(position),
        format_duration(duration),
        (volume * 100.).floor()
    )
}

pub async fn volume(ctx: &Context, guild_id: GuildId, new_volume: f32) -> String {
    let current_track = CURRENT_TRACKS.lock().await;

    let track_handle = match current_track.get(&guild_id.0) {
        Some(track_handle) => track_handle,
        _ => return "Não há nenhuma música tocando.".to_string(),
    };

    track_handle.set_volume(new_volume);

    "Volume mudado.".to_string()
}

pub async fn repeat(ctx: &Context, guild_id: GuildId, channel_id: ChannelId) {
    // let current_track_handle = CURRENT_TRACK_HANDLE.lock().await;

    // let track_handle = match current_track_handle.get(&guild_id.0) {
    //     Some(track_handle) => track_handle,
    //     _ => return,
    // };

    // track_handle.get_info().await.unwrap().

    // let mut msg = CreateMessage::default();

    // msg.content("");

    // let map = hashmap_to_json_map(msg.0);

    // ctx.http
    //     .send_message(channel_id.0, &Value::Object(map))
    //     .await;
}

pub async fn skip(ctx: &Context, guild_id: u64, channel_id: u64) -> bool {
    next(&ctx, guild_id, channel_id).await
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

    CURRENT_TRACKS.lock().await.remove(&guild_id);
}
