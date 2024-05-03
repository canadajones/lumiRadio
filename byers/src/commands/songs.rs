use std::time::Duration;

use futures::StreamExt;
use judeharley::db::DbUser;
use poise::serenity_prelude::{
    ButtonStyle, ComponentInteractionDataKind, CreateActionRow, CreateButton, CreateEmbed,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption,
};
use poise::CreateReply;
use tracing_unwrap::{OptionExt, ResultExt};

use crate::commands::autocomplete_songs;
use crate::event_handlers::message::update_activity;
use crate::prelude::*;
use judeharley::{
    communication::LiquidsoapCommunication,
    cooldowns::{is_on_cooldown, set_cooldown, UserCooldownKey},
    db::DbSong,
    DiscordTimestamp,
};

/// Song-related commands
#[poise::command(
    slash_command,
    subcommands("request", "playing", "history", "queue", "search"),
    subcommand_required
)]
pub async fn song(_: ApplicationContext<'_>) -> Result<(), Error> {
    Ok(())
}

/// Displays the last 10 songs played
#[poise::command(slash_command)]
pub async fn history(ctx: ApplicationContext<'_>) -> Result<(), Error> {
    let data = ctx.data;

    if let Some(guild_id) = ctx.guild_id() {
        update_activity(data, ctx.author().id, ctx.channel_id(), guild_id).await?;
    }

    let last_songs = DbSong::last_10_songs(&data.db).await?;

    let description = last_songs
        .into_iter()
        .enumerate()
        .map(|(i, song)| format!("{}. {} - {}\n", i + 1, song.album, song.title))
        .collect::<Vec<_>>()
        .join("\n");

    ctx.send(
        CreateReply::default().embed(
            CreateEmbed::new()
                .title("Song History")
                .description(format!("```\n{}\n```", description)),
        ),
    )
    .await?;

    Ok(())
}

/// Displays the currently playing song
#[poise::command(slash_command)]
pub async fn playing(ctx: ApplicationContext<'_>) -> Result<(), Error> {
    let data = ctx.data;

    if let Some(guild_id) = ctx.guild_id() {
        update_activity(data, ctx.author().id, ctx.channel_id(), guild_id).await?;
    }

    let Some(current_song) = DbSong::last_played_song(&data.db).await? else {
        ctx.send(
            CreateReply::default().embed(
                CreateEmbed::new()
                    .title("Currently Playing")
                    .description("Nothing is currently playing!"),
            ),
        )
        .await?;
        return Ok(());
    };
    let play_count = current_song.played(&data.db).await?;
    let request_count = current_song.requested(&data.db).await?;

    ctx.send(
        CreateReply::default().embed(CreateEmbed::new().title("Currently Playing").description(
            format!(
                "{} - {}\n\nThis song has been played {} times and requested {} times.",
                current_song.album, current_song.title, play_count, request_count
            ),
        )),
    )
    .await?;

    Ok(())
}

/// Displays the current queue
#[poise::command(slash_command)]
pub async fn queue(ctx: ApplicationContext<'_>) -> Result<(), Error> {
    let data = ctx.data;

    if let Some(guild_id) = ctx.guild_id() {
        update_activity(data, ctx.author().id, ctx.channel_id(), guild_id).await?;
    }

    let mut comms = data.comms.lock().await;
    let requests = comms.song_requests().await?;

    if requests.is_empty() {
        ctx.send(
            CreateReply::default().embed(
                CreateEmbed::new()
                    .title("Song Queue")
                    .description("There are no songs in the queue!"),
            ),
        )
        .await?;
        return Ok(());
    }

    let queue = requests
        .into_iter()
        .enumerate()
        .map(|(i, song)| {
            format!(
                "{}. {} - {}",
                i + 1,
                song.album.unwrap_or("<no album>".to_string()),
                song.title
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    ctx.send(
        CreateReply::default().embed(
            CreateEmbed::new()
                .title("Song Queue")
                .description(format!("```\n{}\n```", queue)),
        ),
    )
    .await?;

    Ok(())
}

/// Lets you search for a song and then request it
#[poise::command(slash_command)]
pub async fn search(
    ctx: ApplicationContext<'_>,
    #[description = "The song to search for"] search: String,
) -> Result<(), Error> {
    let data = ctx.data;

    if let Some(guild_id) = ctx.guild_id() {
        update_activity(data, ctx.author().id, ctx.channel_id(), guild_id).await?;
    }

    let suggestions = DbSong::search(&data.db, &search)
        .await?
        .into_iter()
        .take(20)
        .collect::<Vec<_>>();

    if suggestions.is_empty() {
        ctx.send(
            CreateReply::default().embed(
                CreateEmbed::new()
                    .title("Song Search")
                    .description("No songs were found matching your search."),
            ),
        )
        .await?;
        return Ok(());
    }

    let suggestion_str = suggestions
        .iter()
        .enumerate()
        .map(|(i, song)| format!("{}. {} - {}", i + 1, song.album, song.title))
        .collect::<Vec<_>>()
        .join("\n");
    let results = suggestions.len();

    let user_cooldown = UserCooldownKey::new(ctx.author().id.get() as i64, "song_request");
    let has_cooldown = is_on_cooldown(&data.redis_pool, user_cooldown).await?;
    let mut song_selection = vec![];
    for song in &suggestions {
        if !song.is_on_cooldown(&data.db).await? {
            let option = CreateSelectMenuOption::new(
                format!("{} - {}", song.album, song.title),
                &song.file_hash,
            );

            song_selection.push(option);
        }
    }

    let mut description = format!(
        "Here are the top {results} results for your search for `{search}`.\n\n```\n{suggestion_str}\n```"
    );
    if let Some(over) = has_cooldown.as_ref() {
        description.push_str(&format!(
            "\n\nYou can request a song again {}.",
            over.relative_time()
        ));
    } else {
        description.push_str("\n\nYou may request one of them now by selecting them below within 2 minutes. Songs that are currently on cooldown will not be selectable.");
    }
    let reply = CreateReply::default().embed(
        CreateEmbed::new()
            .title("Song Search")
            .description(description),
    );
    let reply = if has_cooldown.is_none() {
        reply.components(vec![CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                "song_request",
                CreateSelectMenuKind::String {
                    options: song_selection,
                },
            )
            .placeholder("Select a song")
            .min_values(1)
            .max_values(1),
        )])
    } else {
        reply
    };
    let handle = ctx.send(reply).await?;
    let message = handle.message().await?;
    let Some(mci) = message
        .await_component_interaction(ctx.serenity_context())
        .author_id(ctx.author().id)
        .timeout(Duration::from_secs(120))
        .await
    else {
        handle
            .edit(
                poise::Context::Application(ctx),
                CreateReply::default().components(vec![]),
            )
            .await?;

        return Ok(());
    };

    let song = suggestions
        .into_iter()
        .find(|song| {
            let ComponentInteractionDataKind::StringSelect { values } = &mci.data.kind else {
                return false;
            };

            song.file_hash == values[0]
        })
        .expect_or_log("Failed to find song");

    let _ = {
        let mut comms = data.comms.lock().await;
        comms
            .request_song(&song.file_path)
            .await
            .expect_or_log("Failed to request song")
    };

    song.request(&data.db, ctx.author().id.get()).await?;

    let cooldown_time = chrono::Duration::seconds(5400);
    let over = chrono::Utc::now() + cooldown_time;
    let discord_relative = over.relative_time();

    // r.kind(InteractionResponseType::UpdateMessage)
    //         .interaction_response_data(|b| {
    //             b.embed(|e| {
    //                 e.title("Song Requests")
    //                 .description(format!(r#""{} - {}" requested! You can request again in 1 and 1/2 hours ({discord_relative})."#, &song.album, &song.title))
    //             })
    //             .components(|c| c)
    //         })
    mci.create_response(
        ctx.serenity_context(),
        CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::new()
                .embed(
                    CreateEmbed::new()
                        .title("Song Requests")
                        .description(format!(
                            "{} - {} requested! You can request again in 1 and 1/2 hours ({})",
                            &song.album, &song.title, discord_relative
                        )),
                )
                .components(vec![]),
        ),
    )
    .await?;

    set_cooldown(&data.redis_pool, user_cooldown, 90 * 60).await?;

    Ok(())
}

/// Requests a song for the radio
#[poise::command(slash_command)]
pub async fn request(
    ctx: ApplicationContext<'_>,
    #[description = "The song to request"]
    #[rest]
    #[autocomplete = "autocomplete_songs"]
    song: String,
) -> Result<(), Error> {
    let data = ctx.data();

    if let Some(guild_id) = ctx.guild_id() {
        update_activity(data, ctx.author().id, ctx.channel_id(), guild_id).await?;
    }

    let user_cooldown = UserCooldownKey::new(ctx.author().id.get() as i64, "song_request");
    if let Some(over) = is_on_cooldown(&data.redis_pool, user_cooldown).await? {
        ctx.send(
            CreateReply::default().embed(CreateEmbed::new().title("Song Requests").description(
                format!("You can request a song again {}.", over.relative_time()),
            )),
        )
        .await?;
        return Ok(());
    }

    let song = DbSong::fetch_from_hash(&data.db, &song).await?;

    let Some(song) = song else {
        ctx.send(
            CreateReply::default()
                .content("Song not found.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let Some(currently_playing) = DbSong::last_played_song(&data.db).await? else {
        ctx.send(
            CreateReply::default()
                .embed(
                    CreateEmbed::new()
                        .title("Song Requests")
                        .description("Nothing is currently playing!"),
                )
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };
    if currently_playing.file_hash == song.file_hash {
        ctx.send(
            CreateReply::default()
                .embed(
                    CreateEmbed::new()
                        .title("Song Requests")
                        .description("This song is currently playing!"),
                )
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let last_played = song.last_requested(&data.db).await?;
    let cooldown_time = if song.duration < 300.0 {
        chrono::Duration::seconds(1800)
    } else if song.duration < 600.0 {
        chrono::Duration::seconds(3600)
    } else {
        chrono::Duration::seconds(5413)
    };

    let over = last_played + cooldown_time;

    if over > chrono::Utc::now().naive_utc() {
        // b.embed(|e| {
        //     e.title("Song Requests").description(format!(
        //         "This song has been requested recently. You can request this song again {}",
        //         over.relative_time()
        //     ))
        // })
        ctx.send(
            CreateReply::default().embed(CreateEmbed::new().title("Song Requests").description(
                format!(
                    "This song has been requested recently. You can request this song again {}",
                    over.relative_time()
                ),
            )),
        )
        .await?;
        return Ok(());
    }

    let _ = {
        let mut comms = data.comms.lock().await;
        comms
            .request_song(&song.file_path)
            .await
            .expect_or_log("Failed to request song")
    };

    song.request(&data.db, ctx.author().id.get()).await?;

    let cooldown_time = chrono::Duration::seconds(5400);
    let over = chrono::Utc::now() + cooldown_time;
    let discord_relative = over.relative_time();

    let handle = ctx.send(
        CreateReply::default()
            .embed(
                CreateEmbed::new()
                    .title("Song Requests")
                    .description(format!(
                        r#""{} - {}" requested! You can request again in 1 and 1/2 hours ({discord_relative})."#,
                        &song.album, &song.title
                    )),
            )
            .components(vec![
                CreateActionRow::Buttons(vec![
                    CreateButton::new("song_request_favourite")
                        .label("Mark as favourite")
                        .style(ButtonStyle::Primary)
                        .emoji('⭐'),
                    CreateButton::new("song_request_unfavourite")
                        .label("Unmark as favourite")
                        .style(ButtonStyle::Danger),
                ])
            ])
    )
        .await
        .map_err(|e| {
            tracing::error!("Failed to send message: {}", e);
            e
        })?;

    set_cooldown(&data.redis_pool, user_cooldown, 90 * 60).await?;

    let message = handle.message().await?;
    while let Some(mci) = message
        .await_component_interactions(ctx.serenity_context())
        .timeout(Duration::from_secs(60))
        .stream()
        .next()
        .await
    {
        let interaction_user = DbUser::fetch_or_insert(&data.db, mci.user.id.get() as i64).await?;

        match mci.data.custom_id.as_ref() {
            "song_request_favourite" => {
                interaction_user
                    .mark_as_favourite(&song.file_hash, &data.db)
                    .await
                    .expect_or_log("Failed to mark as favourite");
                mci.create_response(
                    ctx.serenity_context(),
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .ephemeral(true)
                            .content("Marked as favourite!"),
                    ),
                )
                .await?;
            }
            "song_request_unfavourite" => {
                interaction_user
                    .mark_as_unfavourited(&song.file_hash, &data.db)
                    .await
                    .expect_or_log("Failed to unmark as favourite");
                mci.create_response(
                    ctx.serenity_context(),
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .ephemeral(true)
                            .content("Unmarked as favourite!"),
                    ),
                )
                .await?;
            }
            _ => unreachable!(),
        }
    }

    Ok(())
}
