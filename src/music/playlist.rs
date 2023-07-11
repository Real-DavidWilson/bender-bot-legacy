use std::{collections::HashMap, sync::Arc};

use lazy_static::lazy_static;
use serenity::{
    futures::lock::{Mutex, MutexGuard},
    model::{
        prelude::{Channel, Guild},
        user::User,
    },
    prelude::Context,
};
use songbird::input::Input;

use super::player::{format_duration, MediaInfo};

type PlaylistResult<T> = Result<T, PlaylistError>;

pub struct PlaylistItem {
    pub ctx: Context,
    pub uri: String,
    pub guild: Guild,
    pub channel: Channel,
    pub author: User,
    pub source: Input,
}

pub struct PlaylistInfo {
    pub limit_per_page: usize,
    pub total_pages: usize,
    pub total_tracks: usize,
    pub items: Vec<PlaylistItemInfo>,
}

#[derive(Debug)]
pub struct PlaylistItemInfo {
    pub index: usize,
    pub media_info: MediaInfo,
}

unsafe impl Send for PlaylistItem {}

const PLAYLIST_LIMIT: usize = 50;

lazy_static! {
    static ref PLAYLISTS: Mutex<HashMap<u64, Vec<PlaylistItem>>> = Mutex::new(HashMap::new());
}

#[derive(Debug)]
pub enum PlaylistError {
    PlaylistFull,
    PlaylistEmpty,
}

pub async fn insert(guild_id: u64, item: PlaylistItem) -> PlaylistResult<()> {
    prepare_playlist(guild_id).await;

    let mut guilds_playlist = PLAYLISTS.lock().await;
    let mut guild_playlist = guilds_playlist.get_mut(&guild_id).unwrap();

    if guild_playlist.len() == PLAYLIST_LIMIT {
        return Err(PlaylistError::PlaylistFull);
    }

    guild_playlist.push(item);

    Ok(())
}

pub async fn reset(guild_id: u64) -> Option<()> {
    let mut guilds_playlist = PLAYLISTS.lock().await;

    guilds_playlist.remove(&guild_id)?;

    Some(())
}

async fn prepare_playlist(guild_id: u64) {
    let mut guilds_playlist = PLAYLISTS.lock().await;
    let guild_playlist = guilds_playlist.get(&guild_id);

    if guild_playlist.is_some() {
        return;
    }

    guilds_playlist.insert(guild_id, Vec::new());
}

pub async fn next(guild_id: u64) -> Option<PlaylistItem> {
    let mut playlists = PLAYLISTS.lock().await;

    if playlists.is_empty() {
        return None;
    }

    let playlist = playlists.get_mut(&guild_id)?;

    let item = playlist.remove(0);

    if playlist.len() == 0 {
        playlists.remove(&guild_id).unwrap();
    }

    Some(item)
}

pub async fn info(guild_id: u64, mut page: usize, limit: usize) -> Option<PlaylistInfo> {
    let mut guilds_playlist = PLAYLISTS.lock().await;
    let guild_playlist = guilds_playlist.get(&guild_id)?;

    let mut items: Vec<PlaylistItemInfo> = vec![];

    if page == 0 {
        page = 1;
    }

    let mut offset = (page - 1) * limit;
    let mut max_length = offset + limit;

    if limit == 0 {
        return None;
    }

    if guild_playlist.len() == 0 {
        return None;
    }

    if guild_playlist.len() <= offset {
        return None;
    }

    if guild_playlist.len() <= max_length {
        max_length = guild_playlist.len();
    }

    for (i, item) in guild_playlist[offset..max_length].iter().enumerate() {
        let metadata = item.source.metadata.clone();

        let title = metadata.title.unwrap();
        let thumb = metadata.thumbnail.unwrap();
        let artist = metadata.artist.unwrap();
        let video_duration = metadata.duration;
        let url = metadata.source_url.unwrap();
        let duration = format_duration(metadata.duration);

        items.push(PlaylistItemInfo {
            index: offset + i,
            media_info: MediaInfo {
                title,
                thumb,
                artist,
                video_duration,
                url,
                duration,
            },
        })
    }

    let mut info = PlaylistInfo {
        items,
        limit_per_page: limit,
        total_pages: guild_playlist.len() / limit + 1,
        total_tracks: guild_playlist.len(),
    };

    return Some(info);
}
