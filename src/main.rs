#![allow(unused)]
#[macro_use]
extern crate dotenv_codegen;

extern crate chrono;
extern crate redis;

use std::ops::{Sub, SubAssign};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chrono::{Local, TimeZone};
use serenity::async_trait;
use serenity::client::{Client, Context};
use serenity::framework::standard::macros::hook;
use serenity::framework::standard::{
    macros::{command, group},
    CommandResult, StandardFramework,
};
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::prelude::interaction::{Interaction, InteractionResponseType};
use serenity::model::prelude::{GuildId, Message, Ready};
use serenity::prelude::{EventHandler, GatewayIntents};
use serenity_additions::ephemeral_message::EphemeralMessage;
use serenity_additions::RegisterAdditions;
use songbird::packet::pnet::types::u1;
use songbird::SerenityInit;
use tracing_subscriber::fmt::format;

use backup::*;
use chat::*;
use commands::*;
use music::*;
use network::*;
use status::*;

mod backup;
mod chat;
mod commands;
mod music;
mod network;
mod status;

const TOKEN: &'static str = dotenv!("DISCORD_TOKEN");

#[hook]
async fn unknown_command(ctx: &Context, msg: &Message, unknown_command_name: &str) {
    msg.reply(
        &ctx.http,
        "Não conheço este comando, tem certeza de que existe?",
    )
    .await
    .unwrap();
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let command = interaction.application_command().unwrap();
        let typing = command.channel_id.start_typing(&ctx.http).unwrap();

        command
            .defer(&ctx.http)
            .await
            .expect("Não foi possível adiar a resposta da interação.");

        let content = match command.data.name.as_str() {
            "ping" => commands::ping::run(&command.data.options).await,
            "play" => commands::play::run(&ctx, &command).await,
            _ => "Sem implementação para este comando.".to_string(),
        };

        command
            .edit_original_interaction_response(&ctx.http, |response| response.content(content))
            .await
            .expect("Não foi possível enviar resposta para a interação.");

        typing.stop().unwrap();
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        let guilds = ctx.http.get_guilds(None, None).await.unwrap();

        for guild in guilds {
            let guild_id = guild.id;

            let commands = guild_id
                .as_ref()
                .set_application_commands(&ctx.http, |commands| {
                    commands
                        .create_application_command(|command| commands::ping::register(command))
                        .create_application_command(|command| commands::play::register(command))
                })
                .await
                .unwrap();
        }
    }
}

#[tokio::main]
async fn main() {
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!"))
        .unrecognised_command(unknown_command)
        .group(&CHAT_GROUP)
        .group(&MUSIC_GROUP)
        .group(&NETWORK_GROUP)
        .group(&BACKUP_GROUP)
        .group(&STATUS_GROUP);

    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::GUILDS
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILD_MESSAGES;

    let mut client = Client::builder(TOKEN, intents)
        .event_handler(Handler)
        .register_songbird()
        .register_serenity_additions()
        .framework(framework)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}
