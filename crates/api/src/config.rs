#[cfg(feature = "server")]
use figment::{Figment, providers::Env};
use {serde::Deserialize, std::sync::LazyLock, url::Url};

#[derive(Deserialize)]
pub struct Config {
    // pub session_secret: SecretString,
    pub database_url: Url,
}

impl Config {
    fn new() -> Self {
        Figment::new().merge(Env::raw()).extract().unwrap()
    }
}

static CONFIG: LazyLock<Config> = LazyLock::new(|| Config::new());

pub fn config() -> &'static Config {
    &CONFIG
}
