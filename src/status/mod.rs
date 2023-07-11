use std::ops::Sub;

use memory_stats::memory_stats;
use serenity::{
    framework::standard::{
        macros::{command, group},
        CommandResult,
    },
    model::prelude::Message,
    prelude::Context,
};

use chrono::Local;

#[group]
#[commands(usage)]
pub struct Status;

#[command]
pub async fn usage(ctx: &Context, msg: &Message) -> CommandResult {
    if let Some(usage) = memory_stats() {
        msg.reply(
            &ctx.http,
            format!(
                "Memória física: {}mb\nMemória Virtual: {}mb",
                usage.physical_mem / 1000000,
                usage.virtual_mem / 1000000
            ),
        )
        .await?;
    }

    Ok(())
}
