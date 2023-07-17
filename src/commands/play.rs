use std::time::Instant;

use chrono::Local;
use serenity::{
    builder::{CreateApplicationCommand, CreateApplicationCommandOption},
    model::{
        prelude::{
            command::{CommandOptionType, CommandType},
            interaction::application_command::{
                ApplicationCommandInteraction, CommandDataOption, CommandDataOptionValue,
            },
            ChannelType, GuildId,
        },
        user::User,
    },
    prelude::Context,
};

use crate::music::{
    player::{self, PlayerError, PlayerStatus},
    send_media_message,
};

pub async fn run(ctx: &Context, command: &ApplicationCommandInteraction) -> String {
    let guild_id = command.guild_id.unwrap();
    let channel_id = command.channel_id;
    let options = command.data.options.clone();

    let uri_option = options.get(0).unwrap().resolved.as_ref().unwrap();

    let uri: String = match uri_option {
        CommandDataOptionValue::String(uri) => uri.to_owned(),
        _ => "".to_string(),
    };

    let member = command.member.as_ref().unwrap().clone();

    let status = player::add(ctx.clone(), uri, guild_id, channel_id, member).await;

    let content: &str = match status {
        Ok(PlayerStatus::Queued) => "A sua música foi adicionada na playlist.",
        Ok(PlayerStatus::Playing(media_info)) => {
            send_media_message(
                &ctx,
                command.member.as_ref().unwrap(),
                channel_id,
                media_info.to_owned(),
            )
            .await;

            "Sua musga ta tocando porra!!!"
        }
        Err(PlayerError::MusicNotFound) => "Musica não encontrada.",
        Err(PlayerError::UserOffVoiceChannel) => "Você precisa estar em um canal de voz.",
        _ => "Não foi possível obter um resultado.",
    };

    content.to_string()
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("play")
        .kind(CommandType::ChatInput)
        .create_option(|option| {
            option
                .name("uri")
                .description("Parametro de busca.")
                .kind(CommandOptionType::String)
                .required(true)
        })
        .description("Toca músicas a partir do youtube.")
}
