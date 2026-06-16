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
    #[serde(default)]
    pub validation: domain::services::ValidationConfig,
    #[serde(default)]
    pub deal_timeouts: DealTimeoutSettings,
    #[serde(default)]
    pub deal_timeout_worker: DealTimeoutWorkerSettings,
    #[serde(default)]
    pub messages: MessagesSettings,
    #[serde(default)]
    pub notifications: NotificationSettings,
    #[serde(default)]
    pub trust_score: TrustScoreSettings,
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

#[derive(Debug, Default, Deserialize, Clone)]
pub struct NotificationSettings {
    #[serde(default = "default_locale")]
    pub default_locale: String,
    #[serde(default = "default_notification_worker_enabled")]
    pub worker_enabled: bool,
    #[serde(default = "default_notification_worker_interval_seconds")]
    pub worker_interval_seconds: u64,
    #[serde(default = "default_notification_worker_batch_size")]
    pub worker_batch_size: usize,
    #[serde(default = "default_email_max_retries")]
    pub push_max_retries: u32,
    #[serde(default = "default_email_retry_base_delay_ms")]
    pub push_retry_base_delay_ms: u64,
    #[serde(default = "default_email_retry_max_delay_ms")]
    pub push_retry_max_delay_ms: u64,
}

fn default_locale() -> String {
    "en".to_string()
}

fn default_notification_worker_enabled() -> bool {
    true
}

fn default_notification_worker_interval_seconds() -> u64 {
    30
}

fn default_notification_worker_batch_size() -> usize {
    100
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct TrustScoreSettings {
    #[serde(default)]
    pub weights: TrustScoreWeights,
    #[serde(default)]
    pub tiers: TrustTierSettings,
    #[serde(default)]
    pub decay: TrustDecaySettings,
    #[serde(default)]
    pub cold_start: TrustColdStartSettings,
    #[serde(default)]
    pub nightly_job: TrustNightlyJobSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TrustScoreWeights {
    #[serde(default = "default_weight_transaction_history")]
    pub transaction_history: f64,
    #[serde(default = "default_weight_review_ratings")]
    pub review_ratings: f64,
    #[serde(default = "default_weight_profile_completeness")]
    pub profile_completeness: f64,
    #[serde(default = "default_weight_verification_level")]
    pub verification_level: f64,
    #[serde(default = "default_weight_response_rate")]
    pub response_rate: f64,
    #[serde(default = "default_weight_dispute_history")]
    pub dispute_history: f64,
    #[serde(default = "default_weight_longevity")]
    pub longevity: f64,
    #[serde(default = "default_weight_community")]
    pub community: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TrustTierSettings {
    #[serde(default = "default_tier_bronze_max")]
    pub bronze_max: i32,
    #[serde(default = "default_tier_silver_max")]
    pub silver_max: i32,
    #[serde(default = "default_tier_gold_max")]
    pub gold_max: i32,
    #[serde(default = "default_tier_platinum_max")]
    pub platinum_max: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TrustDecaySettings {
    #[serde(default = "default_decay_inactivity_threshold_days")]
    pub inactivity_threshold_days: i64,
    #[serde(default = "default_decay_inactivity_monthly_penalty")]
    pub inactivity_monthly_penalty: f64,
    #[serde(default = "default_decay_penalty_halve_months")]
    pub penalty_halve_months: i64,
    #[serde(default = "default_decay_penalty_expire_months")]
    pub penalty_expire_months: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TrustColdStartSettings {
    #[serde(default = "default_cold_start_neutral_score")]
    pub neutral_score: f64,
    #[serde(default = "default_cold_start_review_threshold")]
    pub review_threshold: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TrustNightlyJobSettings {
    #[serde(default = "default_nightly_job_enabled")]
    pub enabled: bool,
    #[serde(default = "default_nightly_job_interval_seconds")]
    pub interval_seconds: u64,
    #[serde(default = "default_nightly_job_batch_size")]
    pub batch_size: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MessagesSettings {
    #[serde(default)]
    pub encryption_key: SecretString,
    #[serde(default = "default_max_pinned_per_conversation")]
    pub max_pinned_per_conversation: i32,
}

impl Default for TrustScoreWeights {
    fn default() -> Self {
        Self {
            transaction_history: default_weight_transaction_history(),
            review_ratings: default_weight_review_ratings(),
            profile_completeness: default_weight_profile_completeness(),
            verification_level: default_weight_verification_level(),
            response_rate: default_weight_response_rate(),
            dispute_history: default_weight_dispute_history(),
            longevity: default_weight_longevity(),
            community: default_weight_community(),
        }
    }
}

impl Default for TrustTierSettings {
    fn default() -> Self {
        Self {
            bronze_max: default_tier_bronze_max(),
            silver_max: default_tier_silver_max(),
            gold_max: default_tier_gold_max(),
            platinum_max: default_tier_platinum_max(),
        }
    }
}

impl Default for TrustDecaySettings {
    fn default() -> Self {
        Self {
            inactivity_threshold_days: default_decay_inactivity_threshold_days(),
            inactivity_monthly_penalty: default_decay_inactivity_monthly_penalty(),
            penalty_halve_months: default_decay_penalty_halve_months(),
            penalty_expire_months: default_decay_penalty_expire_months(),
        }
    }
}

impl Default for TrustColdStartSettings {
    fn default() -> Self {
        Self {
            neutral_score: default_cold_start_neutral_score(),
            review_threshold: default_cold_start_review_threshold(),
        }
    }
}

impl Default for TrustNightlyJobSettings {
    fn default() -> Self {
        Self {
            enabled: default_nightly_job_enabled(),
            interval_seconds: default_nightly_job_interval_seconds(),
            batch_size: default_nightly_job_batch_size(),
        }
    }
}

impl Default for MessagesSettings {
    fn default() -> Self {
        Self {
            encryption_key: SecretString::from(""),
            max_pinned_per_conversation: default_max_pinned_per_conversation(),
        }
    }
}

fn default_weight_transaction_history() -> f64 {
    0.25
}
fn default_weight_review_ratings() -> f64 {
    0.20
}
fn default_weight_profile_completeness() -> f64 {
    0.10
}
fn default_weight_verification_level() -> f64 {
    0.15
}
fn default_weight_response_rate() -> f64 {
    0.10
}
fn default_weight_dispute_history() -> f64 {
    0.10
}
fn default_weight_longevity() -> f64 {
    0.05
}
fn default_weight_community() -> f64 {
    0.05
}

fn default_tier_bronze_max() -> i32 {
    39
}
fn default_tier_silver_max() -> i32 {
    59
}
fn default_tier_gold_max() -> i32 {
    74
}
fn default_tier_platinum_max() -> i32 {
    100
}

fn default_decay_inactivity_threshold_days() -> i64 {
    180
}
fn default_decay_inactivity_monthly_penalty() -> f64 {
    2.0
}
fn default_decay_penalty_halve_months() -> i64 {
    12
}
fn default_decay_penalty_expire_months() -> i64 {
    24
}

fn default_cold_start_neutral_score() -> f64 {
    50.0
}
fn default_cold_start_review_threshold() -> i32 {
    3
}

fn default_nightly_job_enabled() -> bool {
    true
}
fn default_nightly_job_interval_seconds() -> u64 {
    86400
}
fn default_nightly_job_batch_size() -> usize {
    1000
}

fn default_max_pinned_per_conversation() -> i32 {
    5
}

#[derive(Debug, Deserialize, Clone)]
pub struct DealTimeoutSettings {
    #[serde(default = "default_draft_timeout_seconds")]
    pub draft_seconds: i64,
    #[serde(default = "default_suggested_timeout_seconds")]
    pub suggested_seconds: i64,
    #[serde(default = "default_pending_review_timeout_seconds")]
    pub pending_review_seconds: i64,
    #[serde(default = "default_negotiating_timeout_seconds")]
    pub negotiating_seconds: i64,
    #[serde(default = "default_awaiting_party_timeout_seconds")]
    pub awaiting_party_seconds: i64,
    #[serde(default = "default_terms_locked_timeout_seconds")]
    pub terms_locked_seconds: i64,
    #[serde(default = "default_committed_timeout_seconds")]
    pub committed_seconds: i64,
    #[serde(default = "default_on_hold_timeout_seconds")]
    pub on_hold_seconds: i64,
    #[serde(default = "default_disputed_timeout_seconds")]
    pub disputed_seconds: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DealTimeoutWorkerSettings {
    #[serde(default = "default_timeout_worker_enabled")]
    pub enabled: bool,
    #[serde(default = "default_timeout_worker_interval_seconds")]
    pub interval_seconds: u64,
    #[serde(default = "default_timeout_worker_batch_size")]
    pub batch_size: usize,
}

fn default_draft_timeout_seconds() -> i64 {
    application::deals::DealTimeoutConfig::default().draft_seconds
}

fn default_suggested_timeout_seconds() -> i64 {
    application::deals::DealTimeoutConfig::default().suggested_seconds
}

fn default_pending_review_timeout_seconds() -> i64 {
    application::deals::DealTimeoutConfig::default().pending_review_seconds
}

fn default_negotiating_timeout_seconds() -> i64 {
    application::deals::DealTimeoutConfig::default().negotiating_seconds
}

fn default_awaiting_party_timeout_seconds() -> i64 {
    application::deals::DealTimeoutConfig::default().awaiting_party_seconds
}

fn default_terms_locked_timeout_seconds() -> i64 {
    application::deals::DealTimeoutConfig::default().terms_locked_seconds
}

fn default_committed_timeout_seconds() -> i64 {
    application::deals::DealTimeoutConfig::default().committed_seconds
}

fn default_on_hold_timeout_seconds() -> i64 {
    application::deals::DealTimeoutConfig::default().on_hold_seconds
}

fn default_disputed_timeout_seconds() -> i64 {
    application::deals::DealTimeoutConfig::default().disputed_seconds
}

fn default_timeout_worker_enabled() -> bool {
    true
}

fn default_timeout_worker_interval_seconds() -> u64 {
    300
}

fn default_timeout_worker_batch_size() -> usize {
    100
}

impl EmailSettings {
    pub fn verification_base_url(&self) -> &str {
        &self.verification_base_url
    }
}

impl Default for DealTimeoutSettings {
    fn default() -> Self {
        Self::from(application::deals::DealTimeoutConfig::default())
    }
}

impl Default for DealTimeoutWorkerSettings {
    fn default() -> Self {
        Self {
            enabled: default_timeout_worker_enabled(),
            interval_seconds: default_timeout_worker_interval_seconds(),
            batch_size: default_timeout_worker_batch_size(),
        }
    }
}

impl From<application::deals::DealTimeoutConfig> for DealTimeoutSettings {
    fn from(config: application::deals::DealTimeoutConfig) -> Self {
        Self {
            draft_seconds: config.draft_seconds,
            suggested_seconds: config.suggested_seconds,
            pending_review_seconds: config.pending_review_seconds,
            negotiating_seconds: config.negotiating_seconds,
            awaiting_party_seconds: config.awaiting_party_seconds,
            terms_locked_seconds: config.terms_locked_seconds,
            committed_seconds: config.committed_seconds,
            on_hold_seconds: config.on_hold_seconds,
            disputed_seconds: config.disputed_seconds,
        }
    }
}

impl From<DealTimeoutSettings> for application::deals::DealTimeoutConfig {
    fn from(settings: DealTimeoutSettings) -> Self {
        Self::new(
            settings.draft_seconds,
            settings.suggested_seconds,
            settings.pending_review_seconds,
            settings.negotiating_seconds,
            settings.awaiting_party_seconds,
            settings.terms_locked_seconds,
            settings.committed_seconds,
            settings.on_hold_seconds,
            settings.disputed_seconds,
        )
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
        assert_eq!(default_max_pinned_per_conversation(), 5);
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
            validation: Default::default(),
            deal_timeouts: Default::default(),
            deal_timeout_worker: Default::default(),
            messages: Default::default(),
            notifications: Default::default(),
            trust_score: Default::default(),
        };

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
            validation: Default::default(),
            deal_timeouts: Default::default(),
            deal_timeout_worker: Default::default(),
            messages: Default::default(),
            notifications: Default::default(),
            trust_score: Default::default(),
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

impl TrustScoreSettings {
    pub fn to_domain_config(&self) -> domain::entities::trust_score::TrustScoreConfig {
        use domain::entities::trust_score::{
            TrustColdStartConfig, TrustDecayConfig, TrustScoreConfig, TrustScoreWeights,
            TrustTierThresholds,
        };

        TrustScoreConfig {
            weights: TrustScoreWeights {
                transaction_history: self.weights.transaction_history,
                review_ratings: self.weights.review_ratings,
                profile_completeness: self.weights.profile_completeness,
                verification_level: self.weights.verification_level,
                response_rate: self.weights.response_rate,
                dispute_history: self.weights.dispute_history,
                longevity: self.weights.longevity,
                community: self.weights.community,
            },
            tiers: TrustTierThresholds {
                silver: self.tiers.bronze_max as f64 + 1.0,
                gold: self.tiers.silver_max as f64 + 1.0,
                platinum: self.tiers.gold_max as f64 + 1.0,
            },
            cold_start: TrustColdStartConfig {
                global_average_review_score: self.cold_start.neutral_score / 20.0,
                min_reviews_before_own_score_dominates: self.cold_start.review_threshold as i64,
            },
            decay: TrustDecayConfig {
                inactivity_penalty_per_30_days: self.decay.inactivity_monthly_penalty,
                max_inactivity_penalty: self.decay.inactivity_monthly_penalty
                    * (self.decay.penalty_expire_months as f64 / 6.0).max(1.0),
            },
            profile_completeness: Default::default(),
        }
    }
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
