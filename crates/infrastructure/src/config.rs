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
    #[serde(default)]
    pub email: EmailSettings,
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

#[derive(Debug, Default, Deserialize, Clone)]
pub struct EmailSettings {
    pub smtp_host: String,
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: SecretString,
    #[serde(default)]
    pub from_address: String,
    #[serde(default)]
    pub from_name: String,
    #[serde(default)]
    pub verification_base_url: String,
    #[serde(default = "default_token_expiry")]
    pub verification_token_expiry_seconds: i64,
    #[serde(default = "default_password_reset_token_expiry")]
    pub password_reset_token_expiry_seconds: i64,
    #[serde(default = "default_email_max_retries")]
    pub email_max_retries: u32,
    #[serde(default = "default_email_retry_base_delay_ms")]
    pub email_retry_base_delay_ms: u64,
    #[serde(default = "default_email_retry_max_delay_ms")]
    pub email_retry_max_delay_ms: u64,
}

fn default_smtp_port() -> u16 {
    587
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

fn default_password_reset_token_expiry() -> i64 {
    3600
}

fn default_email_max_retries() -> u32 {
    3
}

fn default_email_retry_base_delay_ms() -> u64 {
    500
}

fn default_email_retry_max_delay_ms() -> u64 {
    5000
}

impl EmailSettings {
    pub fn verification_base_url(&self) -> &str {
        &self.verification_base_url
    }
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> SecretString {
        self.url.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn defaults_are_reasonable() {
        assert_eq!(default_smtp_port(), 587);
        assert_eq!(default_max_connections(), 10);
        assert_eq!(default_host(), "127.0.0.1");
        assert_eq!(default_port(), 8080);
        assert_eq!(default_log_level(), "info");
        assert_eq!(default_token_expiry(), 86400);
        assert_eq!(default_password_reset_token_expiry(), 3600);
        assert_eq!(default_email_max_retries(), 3);
        assert_eq!(default_email_retry_base_delay_ms(), 500);
        assert_eq!(default_email_retry_max_delay_ms(), 5000);
    }

    #[test]
    fn email_settings_exposes_verification_base_url() {
        let settings = EmailSettings {
            smtp_host: "smtp.example.com".to_string(),
            verification_base_url: "https://app.example.com".to_string(),
            ..Default::default()
        };
        assert_eq!(settings.verification_base_url(), "https://app.example.com");
    }

    #[test]
    fn database_settings_returns_connection_string() {
        let settings = DatabaseSettings {
            url: SecretString::from("postgres://u@host/db"),
            max_connections: 5,
        };
        assert_eq!(
            settings.connection_string().expose_secret(),
            "postgres://u@host/db"
        );
    }

    #[test]
    fn with_database_url_fallback_uses_database_url_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("DATABASE_URL", "postgres://fallback@host/db");

        let settings = DatabaseSettings {
            url: SecretString::from(""),
            max_connections: 5,
        };
        let mut settings = Settings {
            database: settings,
            server: ServerSettings {
                host: default_host(),
                port: default_port(),
            },
            log: Default::default(),
            auth: AuthSettings {
                secret: SecretString::from("secret"),
                token_expiry_seconds: default_token_expiry(),
            },
            email: Default::default(),
        };
        settings.database.url = SecretString::from("");

        let settings = settings.with_database_url_fallback().unwrap();
        assert_eq!(
            settings.database.url.expose_secret(),
            "postgres://fallback@host/db"
        );

        std::env::remove_var("DATABASE_URL");
    }

    #[test]
    fn with_database_url_fallback_keeps_existing_url() {
        let settings = DatabaseSettings {
            url: SecretString::from("postgres://existing@host/db"),
            max_connections: 5,
        };
        let settings = Settings {
            database: settings,
            server: ServerSettings {
                host: default_host(),
                port: default_port(),
            },
            log: Default::default(),
            auth: AuthSettings {
                secret: SecretString::from("secret"),
                token_expiry_seconds: default_token_expiry(),
            },
            email: Default::default(),
        };

        let settings = settings.with_database_url_fallback().unwrap();
        assert_eq!(
            settings.database.url.expose_secret(),
            "postgres://existing@host/db"
        );
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
