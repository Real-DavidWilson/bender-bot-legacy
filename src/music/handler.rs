use std::sync::Arc;

use serde_json::Value;
use serenity::prelude::Context;
use serenity::{
    async_trait, builder::EditMessage, model::prelude::GuildId, utils::hashmap_to_json_map,
};
use songbird::{EventHandler, Songbird};

use super::player;
use super::playlist;

pub struct StopMusicHandle {
    pub ctx: Context,
    pub channel_id: u64,
    pub guild_id: GuildId,
    pub manager: Arc<Songbird>,
}

#[async_trait]
impl<'fut> EventHandler for StopMusicHandle {
    async fn act(&self, _ctx: &songbird::EventContext<'_>) -> Option<songbird::Event> {
        let next_music = playlist::next().await;

        if next_music.is_none() {
            let mut builder = EditMessage::default();

            builder.content("Nenhuma música foi encontrada, saindo...");

            self.manager.leave(self.guild_id).await.unwrap();

            let mut is_playing = player::IS_PLAYING.lock().await;
            *is_playing = false;

            let map = hashmap_to_json_map(builder.0);

            self.ctx
                .http
                .send_message(self.channel_id, &Value::from(map))
                .await
                .unwrap();

            return None
        }

        if next_music.is_some() {
            let mut builder = EditMessage::default();

            builder.content("Tocando a próxima música...");

            let item = next_music.unwrap();

            let play_status = player::play(
                &item.ctx,
                item.source,
                item.guild,
                item.channel.clone(),
                item.author,
            )
            .await;

            let map = hashmap_to_json_map(builder.0);

            let a = self
                .ctx
                .http
                .send_message(item.channel.id().0, &Value::from(map))
                .await
                .unwrap();

            return None
        }

        None
    }
}
