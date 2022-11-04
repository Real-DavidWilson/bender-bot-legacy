use std::sync::Arc;
use std::time::Instant;

use chrono::{format, Duration};
use serde_json::Value;
use serenity::model::prelude::{Channel, ChannelId};
use serenity::prelude::Context;
use serenity::utils::MessageBuilder;
use serenity::{
    async_trait, builder::EditMessage, model::prelude::GuildId, utils::hashmap_to_json_map,
};
use songbird::{EventHandler, Songbird};

use super::player::{self, play_next};
use super::playlist;

use super::send_media_message;

pub struct StopMusicHandle {
    pub ctx: Context,
    pub guild_id: u64,
    pub channel_id: u64,
}

#[async_trait]
impl<'fut> EventHandler for StopMusicHandle {
    async fn act(&self, _ctx: &songbird::EventContext<'_>) -> Option<songbird::Event> {
        let playing_next = play_next(&self.ctx, self.guild_id, self.channel_id).await;

        if playing_next {
            return None;
        }

        let message = self.ctx
            .http
            .get_channel(self.channel_id)
            .await
            .unwrap()
            .id()
            .send_message(&self.ctx.http, |m| {
                m.content("Não há mais nenhuma música na playlist.")
            })
            .await
            .unwrap();

        None
    }
}

pub async fn handle_next() {}
