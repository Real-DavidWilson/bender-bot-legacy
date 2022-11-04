use std::sync::Arc;
use std::time::Instant;

use chrono::Duration;
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

        let mut builder = EditMessage::default();
        
        builder.content("Não há mais nenhuma música na playlist.");

        let map = hashmap_to_json_map(builder.0);

        if !playing_next {
            self.ctx.http.send_message(
                self.channel_id,
                &Value::from(map),
            );
        }

        None
    }
}

pub async fn handle_next() {}
