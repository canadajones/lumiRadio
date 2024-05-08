use poise::CreateReply;
use crate::event_handlers::message::update_activity;
use crate::prelude::*;

/// Chirps back at the person who chirps
#[poise::command(slash_command)]
pub async fn chirp(ctx: ApplicationContext<'_>) -> Result<(), Error> {
    
    if let Some(guild_id) = ctx.guild_id() {
        update_activity(ctx.data, ctx.author().id, ctx.channel_id(), guild_id).await?;
    }

    ctx.send(
        CreateReply::default()
            .content("chirp chirp")
    )
    .await?;

    Ok(())
}