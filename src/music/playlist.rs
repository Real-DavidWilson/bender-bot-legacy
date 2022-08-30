use std::sync::Arc;

use lazy_static::lazy_static;
use serenity::{futures::lock::Mutex, prelude::Context, model::{prelude::{Guild, Channel}, user::User}};
use songbird::input::Input;

type PlaylistResult<T> = Result<T, PlaylistError>;

pub struct PlaylistItem {
    pub ctx: Context,
    pub uri: String,
    pub guild: Guild,
    pub channel: Channel,
    pub author: User,
    pub source: Input
}

unsafe impl Send for PlaylistItem {}

const PLAYLIST_LIMIT: usize = 50;

lazy_static! {
    static ref PLAYLIST: Mutex<Vec<PlaylistItem>> = Mutex::new(vec![]);
}

#[derive(Debug)]
pub enum PlaylistError {
    PlaylistFull,
    PlaylistEmpty,
}

pub async fn insert(item: PlaylistItem) -> PlaylistResult<()> {
    if PLAYLIST.lock().await.len() == PLAYLIST_LIMIT {
        return Err(PlaylistError::PlaylistFull);
    }

    PLAYLIST.lock().await.push(item);

    Ok(())
}

pub async fn next() -> Option<PlaylistItem> {
    if is_empty().await {
        return None;
    }

    Some(PLAYLIST.lock().await.remove(0))
}

pub async fn is_empty() -> bool {
    return PLAYLIST.lock().await.len() == 0;
}
