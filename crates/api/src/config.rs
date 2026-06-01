#[cfg(feature = "server")]
use figment::{Figment, providers::Env};
use {secrecy::SecretString, serde::Deserialize, std::sync::LazyLock, url::Url};

#[derive(Deserialize)]
pub struct Config {
    pub session_secret: SecretString,
    pub database_url: Url,
}

static CONFIG: LazyLock<Config> =
    LazyLock::new(|| Figment::new().merge(Env::raw()).extract().unwrap());

pub fn config() -> &'static Config {
    &*CONFIG
}
