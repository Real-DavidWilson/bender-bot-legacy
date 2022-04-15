use std::borrow::Borrow;
use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use serenity::builder::CreateEmbed;
use serenity::client::bridge::voice::VoiceGatewayManager;
use serenity::{
    async_trait,
    builder::EditMessage,
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    http::CacheHttp,
    model::{channel::Message, id::GuildId},
    utils::{hashmap_to_json_map, Color},
};
use songbird::driver::retry::{Retry, Strategy};
use songbird::input::{Input, Metadata, Restartable};
use songbird::tracks::Track;
use songbird::{tracks::TrackHandle, Call, Config, EventHandler, Songbird, TrackEvent};
use tokio::sync::MutexGuard;
use tracing::Span;
use tracing_futures::Instrument;
use youtube_dl::{SearchOptions, YoutubeDl, YoutubeDlOutput};

struct StopMusicHandle {
    ctx: Context,
    channel_id: u64,
    guild_id: GuildId,
    manager: Arc<Songbird>,
}

#[async_trait]
impl<'fut> EventHandler for StopMusicHandle {
    async fn act(&self, _ctx: &songbird::EventContext<'_>) -> Option<songbird::Event> {
        let mut builder = EditMessage::default();
        builder.content("A musica acabou!");
        let map = hashmap_to_json_map(builder.0);

        self.ctx
            .http()
            .send_message(self.channel_id, &Value::from(map))
            .await
            .unwrap();
        self.manager.leave(self.guild_id).await.unwrap();

        None
    }
}

async fn query_video(uri: String) -> Option<Input> {
    if !uri.starts_with("http") {
        let source = songbird::ytdl(format!("ytsearch1:{}", uri)).await;

        if let Err(_) = source {
            println!("{:?}", source);
            return None;
        }

        return Some(source.unwrap());
    }

    let source = songbird::ytdl(uri).await;

    if let Err(_) = source {
        println!("{:?}", source);
        return None;
    }

    return Some(source.unwrap());
}

fn format_duration(duration: Duration) -> String {
    let naive_duration = chrono::naive::NaiveTime::from_hms(
        (duration.as_secs() as u32 / 60) / 60,
        (duration.as_secs() as u32 / 60) % 60,
        duration.as_secs() as u32 % 60,
    );

    format!("{}", naive_duration.format("%M:%S").to_string())
}

#[command]
#[only_in(guilds)]
pub async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    if let None = channel_id {
        msg.reply(ctx, "Você não está em um canal de voz!")
            .await
            .unwrap();

        return Ok(());
    }

    let connect_to = channel_id.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let uri = args.message().to_string();

    let mut searching_msg = msg.reply(&ctx, "Consultando video...").await.unwrap();

    let source = query_video(uri).await;

    if let None = source {
        searching_msg
            .edit(&ctx, |m| {
                m.content("Não foi possivel encontrar nenhum vídeo!")
            })
            .await
            .unwrap();

        return Ok(());
    }

    let source = source.unwrap();

    let _ = manager.join(guild_id, connect_to).await;

    let handler_lock = manager.get(guild_id);

    if let None = handler_lock {
        searching_msg
            .edit(&ctx.http, |m| {
                m.content("Entre em um canal de voz para tocar uma música!")
            })
            .await
            .unwrap();

        return Ok(());
    }

    let mut handler = handler_lock.as_ref().unwrap().lock().await;

    let title = source.metadata.title.as_ref().unwrap().clone();
    let thumb = source.metadata.thumbnail.as_ref().unwrap().clone();
    let artist = source.metadata.artist.as_ref().unwrap().clone();
    let video_duration = source.metadata.duration.as_ref().unwrap().clone();
    let url = source.metadata.source_url.as_ref().unwrap().clone();

    let duration = format_duration(video_duration);

    let track_handle = handler.play_only_source(source);
    handler.deafen(true).await.unwrap();

    track_handle
        .add_event(
            songbird::Event::Track(TrackEvent::End),
            StopMusicHandle {
                ctx: ctx.clone(),
                channel_id: msg.channel_id.0,
                manager,
                guild_id,
            },
        )
        .unwrap();

    searching_msg.edit(&ctx, |m|
        m.content("").embed(|mut e| {
            e.color(Color::from_rgb(245, 66, 132)).thumbnail(
                "https://www.pngkit.com/png/full/194-1941997_google-play-music-google-play-music-logo-in.png",
            ).image(thumb).field("TOCANDO", title, false).field("ARTISTA", artist, false);

            if video_duration.as_millis() > 0 {
                e.field("DURAÇÃO", duration, false);
            } else {
                e.field("DURAÇÃO", "Em live", false);
            }

            e.field("URL", url, false)
        }),
    ).await.unwrap();

    Ok(())
}

#[command]
#[only_in(guilds)]
pub async fn progress(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(&ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    Ok(())
}

#[command]
#[only_in(guilds)]
pub async fn stop(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = manager.get(guild_id);

    if let None = handler_lock {
        return Ok(());
    }

    manager.leave(guild_id).await.unwrap();
    manager.remove(guild_id).await.unwrap();

    Ok(())
}
