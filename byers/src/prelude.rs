use chrono::{DateTime, TimeZone};
use fred::{
    prelude::{RedisError, RedisErrorKind},
    types::{FromRedis, RedisValue},
};
use lazy_static::lazy_static;
use poise::serenity_prelude as serenity;
use serenity::GatewayIntents;

use crate::app_config::GoogleConfig;

lazy_static! {
    pub static ref INTENTS: GatewayIntents =
        GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
}

pub type Context<'a> = poise::Context<'a, Data, Error>;
pub type ApplicationContext<'a> = poise::ApplicationContext<'a, Data, Error>;
pub type Error = anyhow::Error;
pub type Telnet = std::sync::Arc<tokio::sync::Mutex<telnet::Telnet>>;

pub struct Data {
    pub db: sqlx::PgPool,
    pub telnet: Telnet,
    pub google_config: GoogleConfig,
    pub redis_client: fred::prelude::RedisClient,
    pub subscriber_client: fred::clients::SubscriberClient,
}

pub trait DiscordTimestamp {
    fn short_time(&self) -> String;
    fn long_time(&self) -> String;
    fn short_date(&self) -> String;
    fn long_date(&self) -> String;
    fn long_date_short_time(&self) -> String;
    fn long_date_with_dow_short_time(&self) -> String;
    fn relative_time(&self) -> String;
}

impl DiscordTimestamp for i64 {
    fn short_time(&self) -> String {
        format!("<t:{}:t>", self)
    }

    fn long_time(&self) -> String {
        format!("<t:{}:T>", self)
    }

    fn short_date(&self) -> String {
        format!("<t:{}:d>", self)
    }

    fn long_date(&self) -> String {
        format!("<t:{}:D>", self)
    }

    fn long_date_short_time(&self) -> String {
        format!("<t:{}:f>", self)
    }

    fn long_date_with_dow_short_time(&self) -> String {
        format!("<t:{}:F>", self)
    }

    fn relative_time(&self) -> String {
        format!("<t:{}:R>", self)
    }
}

impl<Tz> DiscordTimestamp for DateTime<Tz>
where
    Tz: TimeZone,
{
    fn short_time(&self) -> String {
        format!("<t:{}:t>", self.timestamp())
    }

    fn long_time(&self) -> String {
        format!("<t:{}:T>", self.timestamp())
    }

    fn short_date(&self) -> String {
        format!("<t:{}:d>", self.timestamp())
    }

    fn long_date(&self) -> String {
        format!("<t:{}:D>", self.timestamp())
    }

    fn long_date_short_time(&self) -> String {
        format!("<t:{}:f>", self.timestamp())
    }

    fn long_date_with_dow_short_time(&self) -> String {
        format!("<t:{}:F>", self.timestamp())
    }

    fn relative_time(&self) -> String {
        format!("<t:{}:R>", self.timestamp())
    }
}

pub struct W<T>(pub T);

pub trait Wrappable {
    fn wrap(self) -> W<Self>
    where
        Self: Sized,
    {
        W(self)
    }
}

impl<T> Wrappable for T where T: Sized {}

impl<T> std::ops::Deref for W<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> AsRef<T> for W<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl TryFrom<W<chrono::NaiveDateTime>> for RedisValue {
    type Error = RedisError;

    fn try_from(value: W<chrono::NaiveDateTime>) -> Result<Self, Self::Error> {
        Ok(RedisValue::Integer(value.0.timestamp()))
    }
}

impl FromRedis for W<chrono::NaiveDateTime> {
    fn from_value(value: fred::types::RedisValue) -> Result<Self, fred::prelude::RedisError> {
        if let fred::types::RedisValue::Integer(i) = value {
            Ok(W(chrono::NaiveDateTime::from_timestamp_opt(i, 0).unwrap()))
        } else {
            Err(fred::prelude::RedisError::new(
                RedisErrorKind::Parse,
                "invalid value",
            ))
        }
    }
}
