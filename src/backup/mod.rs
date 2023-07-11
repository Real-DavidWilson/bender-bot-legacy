use std::borrow::Borrow;

use lazy_static::__Deref;
use serenity::{
    builder::CreateChannel,
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    http::CacheHttp,
    model::prelude::{command, Channel, ChannelType, GuildChannel, Message},
    prelude::Context,
};

use serde::{Deserialize, Serialize};

#[group]
#[commands(backup)]
pub struct Backup;

#[derive(Default, Debug, Serialize, Deserialize)]
struct BackupChannelData {
    id: u64,
    name: String,
    messages_count: u8,
    category_id: Option<u64>,
    position: i64,
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct BackupCategoryData {
    id: u64,
    name: String,
    position: i64,
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct BackupGuildData {
    id: u64,
    name: String,
    channels: Vec<BackupChannelData>,
    categories: Vec<BackupCategoryData>,
}

#[command]
pub async fn backup(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();

    let channels_hashmap = guild.channels(&ctx.http).await.unwrap();

    let mut backup_data = BackupGuildData {
        name: guild.name,
        id: guild.id.0,
        ..Default::default()
    };

    for (channel_id, guild_channel) in channels_hashmap {
        if guild_channel.kind == ChannelType::Category {
            let mut backup_category_info = BackupCategoryData {
                id: channel_id.0,
                name: String::from(guild_channel.name()),
                position: guild_channel.position,
            };

            backup_data.categories.push(backup_category_info);

            continue;
        }

        let mut backup_channel_data = BackupChannelData {
            id: channel_id.0,
            name: String::from(guild_channel.name()),
            messages_count: guild_channel.message_count.unwrap_or(0),
            position: guild_channel.position,
            ..Default::default()
        };

        backup_channel_data.category_id = match guild_channel.parent_id {
            Some(parent_id) => {
                let parent_channel = ctx.http.get_channel(parent_id.0).await.unwrap();
                Some(parent_channel.category().unwrap().id.0)
            }
            None => None,
        };

        backup_data.channels.push(backup_channel_data);
    }

    let backup_value = serde_json::to_value(&backup_data).unwrap();
    let backup_json = serde_json::to_string_pretty(&backup_value).unwrap();

    

    msg.reply(
        &ctx.http,
        format!("sifuder rapa, clonei o backup anterior e fds"),
    )
    .await
    .unwrap();

    Ok(())
}
