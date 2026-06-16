use crate::errors::ApplicationError;
use crate::notifications::dto::NotificationPreferencesDto;
use domain::entities::notification_preference::{
    ChannelPreferences, NotificationPreference, QuietHours,
};
use domain::entities::{NotificationChannel, NotificationType};
use domain::repositories::NotificationPreferenceRepository;
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone)]
pub struct UpdateNotificationPreferences {
    repo: Arc<dyn NotificationPreferenceRepository>,
}

impl UpdateNotificationPreferences {
    pub fn new(repo: Arc<dyn NotificationPreferenceRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        user_id: Uuid,
        dto: NotificationPreferencesDto,
    ) -> Result<NotificationPreferencesDto, ApplicationError> {
        if dto.user_id != user_id {
            return Err(ApplicationError::Forbidden);
        }

        let mut per_type = HashMap::new();
        if let Some(map) = dto.per_type.as_object() {
            for (key, value) in map {
                if let Ok(notification_type) = NotificationType::try_from(key.as_str()) {
                    let enabled = value
                        .get("enabled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    let channels: Vec<NotificationChannel> = value
                        .get("channels")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| {
                                    v.as_str()
                                        .and_then(|s| NotificationChannel::try_from(s).ok())
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    per_type.insert(
                        notification_type,
                        domain::entities::notification_preference::TypePreference {
                            enabled,
                            channels,
                        },
                    );
                }
            }
        }

        let prefs = NotificationPreference {
            user_id,
            channels: ChannelPreferences {
                in_app: dto.channels.in_app,
                email: dto.channels.email,
                push: dto.channels.push,
                sms: dto.channels.sms,
            },
            per_type,
            quiet_hours: QuietHours {
                enabled: dto.quiet_hours.enabled,
                start: dto.quiet_hours.start.clone(),
                end: dto.quiet_hours.end.clone(),
                timezone: dto.quiet_hours.timezone.clone(),
                except_critical: dto.quiet_hours.except_critical,
            },
            updated_at: OffsetDateTime::now_utc(),
        };

        self.repo.save(&prefs).await?;

        // Return normalized view.
        let saved = self.repo.get(user_id).await?;
        Ok(NotificationPreferencesDto {
            user_id: saved.user_id,
            channels: crate::notifications::dto::ChannelPreferencesDto {
                in_app: saved.channels.in_app,
                email: saved.channels.email,
                push: saved.channels.push,
                sms: saved.channels.sms,
            },
            per_type: serde_json::to_value(&saved.per_type)
                .unwrap_or(serde_json::Value::Object(Default::default())),
            quiet_hours: crate::notifications::dto::QuietHoursDto {
                enabled: saved.quiet_hours.enabled,
                start: saved.quiet_hours.start.clone(),
                end: saved.quiet_hours.end.clone(),
                timezone: saved.quiet_hours.timezone.clone(),
                except_critical: saved.quiet_hours.except_critical,
            },
        })
    }
}
