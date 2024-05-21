use secrecy::{ExposeSecret, Secret, SecretString};
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;

const APP_ENVIRONMENT_ENV_VAR: &'static str = "APP_ENVIRONMENT";

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: SecretString, // Use SecretString to prevent password from being logged
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub database_name: String,
}

#[derive(Debug, Deserialize)]
pub struct ApplicationSettings {
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> SecretString {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name
        ))
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
        .expect(&format!("Failed to parse {}.", APP_ENVIRONMENT_ENV_VAR));
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
