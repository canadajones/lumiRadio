use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct AppConfig {
    pub discord_token: String,
    pub database_url: String,
    pub redis_url: String,

    pub discord: DiscordConfig,
    pub secret: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DiscordConfig {
    pub client_id: String,
    pub client_secret: String,
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
