use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// A record that a specific user has read a specific message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageRead {
    pub id: Uuid,
    pub message_id: Uuid,
    pub user_id: Uuid,
    pub party_id: Option<Uuid>,
    pub read_at: OffsetDateTime,
}

impl MessageRead {
    pub fn new(id: Uuid, message_id: Uuid, user_id: Uuid, party_id: Option<Uuid>) -> Self {
        Self {
            id,
            message_id,
            user_id,
            party_id,
            read_at: OffsetDateTime::now_utc(),
        }
    }
}
