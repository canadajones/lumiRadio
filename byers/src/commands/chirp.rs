use poise::CreateReply;
use crate::event_handlers::message::update_activity;
use crate::prelude::*;

use rand::{thread_rng, Rng};
use tracing::debug;

/// Chirps back at the person who chirps
#[poise::command(slash_command)]
pub async fn chirp(ctx: ApplicationContext<'_>) -> Result<(), Error> {
    
    if let Some(guild_id) = ctx.guild_id() {
        update_activity(ctx.data, ctx.author().id, ctx.channel_id(), guild_id).await?;
    }

    debug!("chirping!");
    // if author is @canadajones68
    if ctx.author().id == 329162131096338434 {

        debug!("oh hai can");
        
        let rand_val = {
            let mut rng = thread_rng();
            rng.gen_bool(0.1)
        };

        debug!("randval obtained");
        if rand_val {
            ctx.send(
                CreateReply::default()
                .content("chorp")
            )
            .await?;
        }
    }
    else {
        ctx.send(
            CreateReply::default()
                .content("chirp chirp")
        )
        .await?;
    }
    debug!("i has now replied to the chirp");

    Ok(())
}