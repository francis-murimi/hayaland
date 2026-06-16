use crate::errors::ApplicationError;
use crate::notifications::dto::{ChannelPreferencesDto, NotificationPreferencesDto, QuietHoursDto};
use domain::repositories::NotificationPreferenceRepository;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct GetNotificationPreferences {
    repo: Arc<dyn NotificationPreferenceRepository>,
}

impl GetNotificationPreferences {
    pub fn new(repo: Arc<dyn NotificationPreferenceRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        user_id: Uuid,
    ) -> Result<NotificationPreferencesDto, ApplicationError> {
        let prefs = self.repo.get(user_id).await?;
        Ok(NotificationPreferencesDto {
            user_id: prefs.user_id,
            channels: ChannelPreferencesDto {
                in_app: prefs.channels.in_app,
                email: prefs.channels.email,
                push: prefs.channels.push,
                sms: prefs.channels.sms,
            },
            per_type: serde_json::to_value(&prefs.per_type)
                .unwrap_or(serde_json::Value::Object(Default::default())),
            quiet_hours: QuietHoursDto {
                enabled: prefs.quiet_hours.enabled,
                start: prefs.quiet_hours.start.clone(),
                end: prefs.quiet_hours.end.clone(),
                timezone: prefs.quiet_hours.timezone.clone(),
                except_critical: prefs.quiet_hours.except_critical,
            },
        })
    }
}
