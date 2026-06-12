use config::{Config, ConfigError, Environment, File};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub server: ServerSettings,
    #[serde(default)]
    pub log: LogSettings,
    pub auth: AuthSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseSettings {
    #[serde(default)]
    pub url: SecretString,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerSettings {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct LogSettings {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default)]
    pub json: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthSettings {
    pub secret: SecretString,
    #[serde(default = "default_token_expiry")]
    pub token_expiry_seconds: i64,
}

fn default_max_connections() -> u32 {
    10
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_token_expiry() -> i64 {
    86400
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> SecretString {
        self.url.clone()
    }
}

pub fn configuration() -> Result<Settings, ConfigError> {
    let base_path = env::current_dir().expect("failed to determine current directory");
    let config_dir = base_path.join("config");

    let env_name: String = env::var("APP_ENV").unwrap_or_else(|_| "local".into());

    Config::builder()
        .add_source(File::from(config_dir.join("base.toml")).required(false))
        .add_source(File::from(config_dir.join(format!("{env_name}.toml"))).required(false))
        .add_source(
            Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?
        .try_deserialize()
}

impl Settings {
    /// Convenience helper that uses `DATABASE_URL` as a fallback when `APP_DATABASE__URL` is absent.
    pub fn with_database_url_fallback(mut self) -> Result<Self, ConfigError> {
        if self.database.url.expose_secret().is_empty() {
            if let Ok(url) = env::var("DATABASE_URL") {
                self.database.url = SecretString::from(url);
            } else {
                return Err(ConfigError::Message(
                    "database URL is missing: set APP_DATABASE__URL or DATABASE_URL".to_string(),
                ));
            }
        }
        Ok(self)
    }
}
