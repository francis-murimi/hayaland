use async_trait::async_trait;
use domain::entities::{
    notification_preference::{
        ChannelPreferences, NotificationPreference, QuietHours, TypePreference,
    },
    NotificationChannel, NotificationType,
};
use domain::errors::DomainError;
use domain::repositories::NotificationPreferenceRepository;
use sqlx::{Error as SqlxError, PgPool};
use std::collections::HashMap;
use uuid::Uuid;

pub struct PostgresNotificationPreferenceRepository {
    pool: PgPool,
}

impl PostgresNotificationPreferenceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NotificationPreferenceRepository for PostgresNotificationPreferenceRepository {
    async fn get(&self, user_id: Uuid) -> Result<NotificationPreference, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT user_id, channels, per_type, quiet_hours, updated_at
            FROM notification_preferences
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        match row {
            Some(row) => Ok(build_preference(
                row.user_id,
                row.channels,
                row.per_type,
                row.quiet_hours,
                row.updated_at,
            )?),
            None => Ok(NotificationPreference::new(user_id)),
        }
    }

    async fn save(&self, preference: &NotificationPreference) -> Result<(), DomainError> {
        let channels = serde_json::to_value(ChannelPreferencesRow::from(&preference.channels))
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        let per_type = serde_json::to_value(
            preference
                .per_type
                .iter()
                .map(|(k, v)| (k.as_str().to_string(), TypePreferenceRow::from(v)))
                .collect::<HashMap<_, _>>(),
        )
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        let quiet_hours = serde_json::to_value(QuietHoursRow::from(&preference.quiet_hours))
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        sqlx::query!(
            r#"
            INSERT INTO notification_preferences (
                user_id, channels, per_type, quiet_hours, updated_at
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (user_id)
            DO UPDATE SET
                channels = EXCLUDED.channels,
                per_type = EXCLUDED.per_type,
                quiet_hours = EXCLUDED.quiet_hours,
                updated_at = EXCLUDED.updated_at
            "#,
            preference.user_id,
            channels,
            per_type,
            quiet_hours,
            preference.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ChannelPreferencesRow {
    in_app: bool,
    email: bool,
    push: bool,
    sms: bool,
}

impl From<&ChannelPreferences> for ChannelPreferencesRow {
    fn from(p: &ChannelPreferences) -> Self {
        Self {
            in_app: p.in_app,
            email: p.email,
            push: p.push,
            sms: p.sms,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TypePreferenceRow {
    enabled: bool,
    channels: Vec<String>,
}

impl From<&TypePreference> for TypePreferenceRow {
    fn from(p: &TypePreference) -> Self {
        Self {
            enabled: p.enabled,
            channels: p.channels.iter().map(|c| c.as_str().to_string()).collect(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct QuietHoursRow {
    enabled: bool,
    start: String,
    end: String,
    timezone: String,
    except_critical: bool,
}

impl From<&QuietHours> for QuietHoursRow {
    fn from(q: &QuietHours) -> Self {
        Self {
            enabled: q.enabled,
            start: q.start.clone(),
            end: q.end.clone(),
            timezone: q.timezone.clone(),
            except_critical: q.except_critical,
        }
    }
}

fn build_preference(
    user_id: Uuid,
    channels_json: serde_json::Value,
    per_type_json: serde_json::Value,
    quiet_hours_json: serde_json::Value,
    updated_at: time::OffsetDateTime,
) -> Result<NotificationPreference, DomainError> {
    let channels_row: ChannelPreferencesRow = serde_json::from_value(channels_json)
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
    let channels = ChannelPreferences {
        in_app: channels_row.in_app,
        email: channels_row.email,
        push: channels_row.push,
        sms: channels_row.sms,
    };

    let per_type_map: HashMap<String, TypePreferenceRow> = serde_json::from_value(per_type_json)
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
    let mut per_type = HashMap::new();
    for (type_str, pref) in per_type_map {
        if let Ok(t) = NotificationType::try_from(type_str.as_str()) {
            let channels: Vec<NotificationChannel> = pref
                .channels
                .iter()
                .filter_map(|c| NotificationChannel::try_from(c.as_str()).ok())
                .collect();
            per_type.insert(
                t,
                TypePreference {
                    enabled: pref.enabled,
                    channels,
                },
            );
        }
    }

    let quiet_hours_row: QuietHoursRow = serde_json::from_value(quiet_hours_json)
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
    let quiet_hours = QuietHours {
        enabled: quiet_hours_row.enabled,
        start: quiet_hours_row.start,
        end: quiet_hours_row.end,
        timezone: quiet_hours_row.timezone,
        except_critical: quiet_hours_row.except_critical,
    };

    Ok(NotificationPreference {
        user_id,
        channels,
        per_type,
        quiet_hours,
        updated_at,
    })
}

fn map_err(err: SqlxError) -> DomainError {
    DomainError::RepositoryError(err.to_string())
}
