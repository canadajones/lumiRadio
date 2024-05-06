use poise::serenity_prelude as serenity;
use serenity::GatewayIntents;

use std::sync::Arc;

use lazy_static::lazy_static;
use tokio::sync::Mutex;

use judeharley::communication::{ByersUnixStream, LiquidsoapCommunication};

lazy_static! {
    pub static ref INTENTS: GatewayIntents = GatewayIntents::non_privileged()
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MEMBERS;
}

pub type Context<'a> = poise::Context<'a, Data<ByersUnixStream>, Error>;
pub type ApplicationContext<'a> = poise::ApplicationContext<'a, Data<ByersUnixStream>, Error>;
pub type Error = anyhow::Error;

pub struct Data<C>
where
    C: LiquidsoapCommunication,
{
    pub db: judeharley::PgPool,
    pub comms: Arc<Mutex<C>>,
    pub redis_pool: fred::pool::RedisPool,
    pub redis_subscriber: fred::clients::SubscriberClient,
}
