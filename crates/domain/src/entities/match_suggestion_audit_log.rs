use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// An audit entry recording an administrative mutation of a match suggestion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchSuggestionAuditLogEntry {
    pub id: Uuid,
    pub admin_user_id: Option<Uuid>,
    pub action_type: String,
    pub match_suggestion_id: Option<Uuid>,
    pub party_id: Option<Uuid>,
    pub before_snapshot: Option<serde_json::Value>,
    pub after_snapshot: Option<serde_json::Value>,
    pub reason: Option<String>,
    pub created_at: OffsetDateTime,
}

impl MatchSuggestionAuditLogEntry {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Uuid,
        admin_user_id: Option<Uuid>,
        action_type: impl Into<String>,
        match_suggestion_id: Option<Uuid>,
        party_id: Option<Uuid>,
        before_snapshot: Option<serde_json::Value>,
        after_snapshot: Option<serde_json::Value>,
        reason: Option<String>,
    ) -> Self {
        Self {
            id,
            admin_user_id,
            action_type: action_type.into(),
            match_suggestion_id,
            party_id,
            before_snapshot,
            after_snapshot,
            reason,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}
