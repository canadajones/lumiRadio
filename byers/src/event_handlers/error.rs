use chrono::Utc;
use poise::{serenity_prelude::CreateEmbed, CreateReply};
use tracing::error;

use crate::prelude::*;
use judeharley::{communication::ByersUnixStream, prelude::DiscordTimestamp};

type FrameworkError<'a> = poise::FrameworkError<'a, Data<ByersUnixStream>, crate::prelude::Error>;

async fn send_cooldown_embed(
    ctx: Context<'_>,
    remaining_cooldown: core::time::Duration,
) -> Result<(), Error> {
    ctx.send(
        CreateReply::default()
            .embed(
                CreateEmbed::new()
                    .title("You are too fast!")
                    .description(format!(
                        "You can use that command again {}.",
                        (Utc::now().naive_utc()
                            + chrono::Duration::from_std(remaining_cooldown).unwrap())
                        .relative_time()
                    )),
            )
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

pub async fn on_error(error: FrameworkError<'_>) -> Result<(), Error> {
    match error {
        FrameworkError::CooldownHit {
            remaining_cooldown,
            ctx,
            ..
        } => {
            send_cooldown_embed(ctx, remaining_cooldown).await?;
        }
        FrameworkError::Command { error, ctx, .. } => {
            let err_str = error.to_string();
            error!("Error in command: {}", err_str);
            sentry::add_breadcrumb(BreadcrumbableContext(ctx).as_breadcrumbs().await);
            sentry_anyhow::capture_anyhow(&error);
            ctx.say(err_str).await?;
        }
        _ => {
            poise::builtins::on_error(error).await?;
        }
    }

    Ok(())
}
