use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use sqlx::ConnectOptions;

use crate::domain::{Email, ParseEmailError, ParseUrlError, Url};

const APP_ENVIRONMENT_ENV_VAR: &str = "APP_ENVIRONMENT";

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    pub email_client: EmailClientSettings,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: SecretString, // Use SecretString to prevent password from being logged
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub database_name: String,
    // Whether the connection should be encrypted or not
    pub require_ssl: bool,
}

impl DatabaseSettings {
    pub fn with_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };

        PgConnectOptions::new()
            .host(&self.host)
            .port(self.port)
            .username(&self.username)
            .password(self.password.expose_secret())
            .database(&self.database_name)
            .ssl_mode(ssl_mode)
            // Logging level
            .log_statements(tracing_log::log::LevelFilter::Trace)
    }
}

#[derive(Debug, Deserialize)]
pub struct ApplicationSettings {
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    pub authorization_token: SecretString,
}

impl EmailClientSettings {
    pub fn sender(&self) -> Result<Email, ParseEmailError> {
        Email::parse(self.sender_email.clone())
    }

    pub fn url(&self) -> Result<Url, ParseUrlError> {
        Url::parse(self.base_url.clone())
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine current directory.");
    let config_dir = base_path.join("config");

    // Detect the running environment.
    // Default to `local` if unspecified.
    let environment: Environment = std::env::var(APP_ENVIRONMENT_ENV_VAR)
        .unwrap_or_else(|_| Environment::Local.to_string())
        .try_into()
        .unwrap_or_else(|_| panic!("Failed to parse {}.", APP_ENVIRONMENT_ENV_VAR));
    let environment_filename = format!("{}.yaml", environment);

    // Initialize configuration reader.
    let settings = config::Config::builder()
        .add_source(config::File::from(config_dir.join("base.yaml")))
        .add_source(config::File::from(config_dir.join(environment_filename)))
        // Add in settings from environment variables (with a prefix of APP and '__' as separator)
        // E.g. `APP_APPLICATION__PORT=5001 would set `Settings.application.port`
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;

    // Deserialize configuration values into Settings.
    settings.try_deserialize::<Settings>()
}

#[derive(strum_macros::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Environment {
    Local,
    Production,
}

impl TryFrom<String> for Environment {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. Use either `local` or `production`.",
                other
            )),
        }
    }
}
