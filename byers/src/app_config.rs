use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct AppConfig {
    pub discord_token: String,
    pub database_url: String,
    pub redis_url: String,

    pub liquidsoap: LiquidsoapConfig,

    pub google: GoogleConfig,
    pub discord: DiscordConfig,
    pub secret: String,
}

#[derive(Deserialize, Debug)]
pub struct LiquidsoapConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Deserialize, Debug)]
pub struct GoogleConfig {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DiscordConfig {
    pub client_id: String,
    pub client_secret: String,

    pub emoji: EmojiConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct EmojiConfig {
    pub d6_1: String,
    pub d6_2: String,
    pub d6_3: String,
    pub d6_4: String,
    pub d6_5: String,
    pub d6_6: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let config = config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()
            .unwrap();

        config.try_deserialize().unwrap()
    }
}
