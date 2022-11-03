use serde_json::json;
use serenity::{
    client::Context,
    framework::standard::{macros::command, Args, CommandError, CommandResult},
    model::channel::Message,
    prelude::Mentionable,
    utils::MessageBuilder,
};

#[command]
#[only_in(guilds)]
pub async fn clear(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let channel_id = msg.channel_id.0;

    let amount = args.parse::<i32>();

    if amount.is_err() {
        msg.reply(
            &ctx.http,
            "Você deve especificar a quantidade de mensagens.",
        )
        .await
        .unwrap();

        return Err(CommandError::from("Ocorreu um erro."));
    }

    let mut amount = amount.unwrap();

    if amount < 2 || amount > 100 {
        msg.reply(
            &ctx.http,
            "A quantidade de mensagens deve ser entre 2 e 100.",
        )
        .await
        .unwrap();

        return Err(CommandError::from("Ocorreu um erro."));
    }

    amount += 1;

    let mut messages_ids = ctx
        .http
        .get_messages(
            msg.channel_id.0,
            &format!(
                "?{}",
                querystring::stringify(vec![("limit", &amount.to_string())])
            ),
        )
        .await
        .unwrap()
        .into_iter()
        .filter(|message| message.id.0 != msg.id.0)
        .map(|message| format!("{}", message.id.0))
        .collect::<Vec<String>>();

    if messages_ids.len() == 0 {
        msg.reply(&ctx.http, "Este canal não possui nenhuma mensagem.")
            .await
            .unwrap();

        return Err(CommandError::from("Ocorreu um erro."));
    }

    let map = json!({ "messages": messages_ids });

    let res = ctx.http
        .delete_messages(msg.channel_id.0, &map)
        .await;

    if res.is_err() {
        msg.reply(&ctx.http, "Não foi possível deletar as mensagens desse canal.")
            .await
            .unwrap();

        return Err(CommandError::from("Ocorreu um erro."));
    }

    msg.delete(&ctx.http).await.unwrap();

    Ok(())
}
