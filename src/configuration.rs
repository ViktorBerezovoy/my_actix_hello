use config::Config;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use url::Url;

use crate::domain::SubscriberEnail;

#[derive(Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    pub email_client: EmailClientSettings,
}

#[derive(Deserialize)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: String,
}

#[derive(Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: SecretString,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> SecretString {
        SecretString::from(format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name
        ))
    }
}

pub enum Environment {
    Local,
    Production,
}
impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Production => "production",
        }
    }
}
impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. Use either 'local' or 'production'",
                other
            )),
        }
    }
}

#[derive(Deserialize)]
pub struct EmailClientSettings {
    pub base_url: Url,
    pub sender_email: SubscriberEnail,
    pub authorization_token: SecretString,
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .map_err(config::ConfigError::NotFound)?;

    let config_file_path = format!("configuration/{}", environment.as_str());

    Config::builder()
        .add_source(config::File::with_name("configuration/base"))
        .add_source(config::File::with_name(&config_file_path))
        .build()?
        .try_deserialize::<Settings>()
}
