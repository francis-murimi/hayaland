use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// A push-notification token registered for a user device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PushToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_token: String,
    pub provider: String,
    pub device_type: Option<String>,
    pub created_at: OffsetDateTime,
    pub last_used_at: Option<OffsetDateTime>,
}

impl PushToken {
    pub fn new(
        id: Uuid,
        user_id: Uuid,
        device_token: impl Into<String>,
        provider: impl Into<String>,
        device_type: Option<String>,
    ) -> Self {
        Self {
            id,
            user_id,
            device_token: device_token.into(),
            provider: provider.into(),
            device_type,
            created_at: OffsetDateTime::now_utc(),
            last_used_at: None,
        }
    }
}
