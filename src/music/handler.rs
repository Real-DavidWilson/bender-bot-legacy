use std::sync::Arc;
use std::time::Instant;

use chrono::{format, Duration};
use serde_json::Value;
use serenity::model::prelude::{Channel, ChannelId};
use serenity::prelude::Context;
use serenity::utils::MessageBuilder;
use serenity::{async_trait, builder::EditMessage, model::prelude::GuildId};
use songbird::{EventHandler, Songbird};

use super::player::{self, next};
use super::playlist;

use super::send_media_message;

pub struct StopMusicHandle {
    pub ctx: Context,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
}

#[async_trait]
impl<'fut> EventHandler for StopMusicHandle {
    async fn act(&self, _ctx: &songbird::EventContext<'_>) -> Option<songbird::Event> {
        let playing_next = next(&self.ctx, self.guild_id.0, self.channel_id.0).await;

        if playing_next {
            return None;
        }

        let channel = self.ctx.http.get_channel(self.channel_id.0).await.unwrap();

        channel
            .id()
            .say(&self.ctx.http, "Não há mais nenhuma música na playlist.")
            .await
            .unwrap();

        None
    }
}

pub async fn handle_next() {}
